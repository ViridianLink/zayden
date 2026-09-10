use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan as _};
use serenity::all::UserId;
use sqlx::PgPool;
use tracing::warn;
use zayden_app::entitlement::{EntitlementService, Tier};
use zayden_core::as_u64;

use crate::error::{HostingError, Result};
use crate::pricing::{self, Plan};
use crate::store;

#[derive(Debug, Clone, Copy)]
pub struct Payment<'a> {
    pub message_id: &'a str,
    pub email_hash: &'a str,
    pub tier_name: Option<&'a str>,
    pub amount_cents: i64,
    pub currency: &'a str,
    pub message: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settlement {
    Duplicate,
    Unmatched(&'static str),
    Settled { server_id: i64, matched_by: &'static str },
}

pub async fn settle(
    pool: &PgPool,
    entitlements: &EntitlementService,
    payment: &Payment<'_>,
) -> Result<Settlement> {
    let amount: i32 = payment
        .amount_cents
        .try_into()
        .map_err(|_e| HostingError::internal("payment amount out of range"))?;

    let first_seen = store::insert_payment(pool, &store::NewPayment {
        kofi_message_id: payment.message_id,
        server_id: None,
        email: None,
        tier_name: payment.tier_name,
        amount_cents: amount,
        currency: payment.currency,
        matched_by: None,
    })
    .await?;

    if !first_seen {
        return Ok(Settlement::Duplicate);
    }

    let Some(price) = ladder_price(payment) else {
        return Ok(Settlement::Unmatched("amount is not on the price ladder"));
    };

    let Some(discord_id) =
        store::discord_id_for_email_hash(pool, payment.email_hash).await?
    else {
        return Ok(Settlement::Unmatched("Ko-fi email is not linked"));
    };

    let owner = UserId::new(as_u64(discord_id));

    let Some((server_id, matched_by)) =
        match_server(pool, entitlements, payment, owner, price).await?
    else {
        return Ok(Settlement::Unmatched("no server matches that tier"));
    };

    store::attach_payment(pool, payment.message_id, server_id, matched_by).await?;

    let row = store::get(pool, server_id).await?;
    store::record_payment(pool, server_id, next_month(row.expires_at())).await?;
    store::notify_paid(pool, server_id).await?;

    Ok(Settlement::Settled { server_id, matched_by })
}

fn ladder_price(payment: &Payment<'_>) -> Option<i64> {
    payment.tier_name.and_then(pricing::price_from_tier_name).or_else(|| {
        pricing::LADDER_CENTS
            .contains(&payment.amount_cents)
            .then_some(payment.amount_cents)
    })
}

async fn match_server(
    pool: &PgPool,
    entitlements: &EntitlementService,
    payment: &Payment<'_>,
    owner: UserId,
    price: i64,
) -> Result<Option<(i64, &'static str)>> {
    if let Some(code) = payment.message.and_then(claim_code)
        && let Some(row) = store::find_by_claim_code(pool, &code).await?
        && row.owner() == owner
    {
        return Ok(Some((row.id, "claim_code")));
    }

    let tier = entitlements.user_tier(owner.get()).await;

    if pricing::plan_for_price(price, tier).is_none() {
        warn!(
            price,
            tier = tier.as_str(),
            "hosting: paid amount buys no plan at this tier"
        );
    }

    let candidates: Vec<_> = store::payable_for_owner(pool, owner)
        .await?
        .into_iter()
        .filter(|row| i64::from(row.price_cents) == price)
        .collect();

    match candidates.as_slice() {
        [] => Ok(None),
        [only] => Ok(Some((only.id, "tier"))),
        // Several servers share this rung; the soonest to lapse is the one the
        // payer is most likely settling. A claim code overrides this.
        [first, ..] => Ok(Some((first.id, "tier_ambiguous"))),
    }
}

fn claim_code(message: &str) -> Option<String> {
    const ALPHABET: &str = "BCDFGHJKMNPQRSTVWXYZ23456789";
    const LEN: usize = 6;

    message.split(|c: char| !c.is_ascii_alphanumeric()).map(str::to_uppercase).find(
        |token| {
            token.chars().count() == LEN
                && token.chars().all(|c| ALPHABET.contains(c))
        },
    )
}

fn next_month(current: Option<Timestamp>) -> Timestamp {
    let now = Timestamp::now();
    let from = current.map_or(now, |t| if t > now { t } else { now });

    from.to_zoned(TimeZone::UTC)
        .checked_add(1.month())
        .map_or(from, |z| z.timestamp())
}

#[must_use]
pub fn plans_at_price(price: i64) -> Vec<(Plan, &'static str)> {
    let mut out = Vec::new();
    for tier in [Tier::Free, Tier::Pro, Tier::Ultra] {
        if let Some(plan) = pricing::plan_for_price(price, tier) {
            out.push((plan, tier.as_str()));
        }
    }
    out
}
