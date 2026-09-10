//! Ko-fi delivers at least once and offers no way to ask what it already sent,
//! so the only defence against a double extension is the message id.

use std::sync::Arc;

use hosting::kofi::{Payment, Settlement, settle};
use hosting::store;
use sqlx::PgPool;
use tokio::sync::broadcast;
use zayden_app::entitlement::EntitlementService;

fn entitlements(pool: &PgPool) -> Arc<EntitlementService> {
    let (events, _rx) = broadcast::channel(8);
    Arc::new(EntitlementService::new(pool.clone(), events))
}

/// Exhaustive rather than a wildcard, so a new `Settlement` variant fails
/// here instead of quietly reading as "not settled".
const fn settled_id(outcome: Option<&Settlement>) -> Option<i64> {
    match outcome {
        Some(Settlement::Settled { server_id, .. }) => Some(*server_id),
        Some(Settlement::Duplicate | Settlement::Unmatched(_)) | None => None,
    }
}

const fn payment<'a>(
    message_id: &'a str,
    tier: &'a str,
    message: Option<&'a str>,
) -> Payment<'a> {
    Payment {
        message_id,
        email_hash: "hash-of-payer",
        tier_name: Some(tier),
        amount_cents: 250,
        currency: "GBP",
        message,
    }
}

#[sqlx::test(migrations = "../../migrations", fixtures("kofi"))]
async fn a_claim_code_picks_the_server_out_of_several_on_the_same_rung(
    pool: PgPool,
) {
    let service = entitlements(&pool);
    let paid = payment("msg-1", "Hosting £2.50", Some("here you go - JKMNPQ"));

    let outcome = settle(&pool, &service, &paid).await.ok();

    assert_eq!(
        outcome,
        Some(Settlement::Settled { server_id: 2, matched_by: "claim_code" })
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("kofi"))]
async fn without_a_claim_code_the_soonest_to_lapse_is_settled(pool: PgPool) {
    let service = entitlements(&pool);
    let paid = payment("msg-2", "Hosting £2.50", None);

    let outcome = settle(&pool, &service, &paid).await.ok();

    assert_eq!(
        settled_id(outcome.as_ref()),
        Some(1),
        "row 1 lapsed yesterday; row 2 has ten days left"
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("kofi"))]
async fn a_replayed_message_id_is_a_no_op(pool: PgPool) {
    let service = entitlements(&pool);
    let paid = payment("msg-3", "Hosting £2.50", Some("BCDFGH"));

    let first = settle(&pool, &service, &paid).await.ok();
    let after_first = store::get(&pool, 1).await.ok().and_then(|r| r.expires_at());

    let second = settle(&pool, &service, &paid).await.ok();
    let after_second = store::get(&pool, 1).await.ok().and_then(|r| r.expires_at());

    assert!(matches!(first, Some(Settlement::Settled { .. })));
    assert_eq!(second, Some(Settlement::Duplicate));
    assert_eq!(
        after_first, after_second,
        "the replay must not buy the payer a second month"
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("kofi"))]
async fn a_payment_settles_the_matching_rung_only(pool: PgPool) {
    let service = entitlements(&pool);
    let paid = payment("msg-4", "Hosting £2.50", None);

    settle(&pool, &service, &paid).await.ok();

    let large = store::get(&pool, 3).await.ok();
    let state = large.map(|r| r.state);

    assert_eq!(
        state.as_deref(),
        Some("active"),
        "the £7.50 server must be untouched by a £2.50 payment"
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("kofi"))]
async fn a_claim_code_belonging_to_someone_else_is_ignored(pool: PgPool) {
    let service = entitlements(&pool);
    // YZ2345 is user 2's server; the payer is user 1.
    let paid = payment("msg-5", "Hosting £2.50", Some("YZ2345"));

    let outcome = settle(&pool, &service, &paid).await.ok();

    let settled = settled_id(outcome.as_ref());

    assert!(settled.is_some(), "the payment should still fall back to a match");
    assert_ne!(
        settled,
        Some(4),
        "a stranger's claim code must not redirect a payment"
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("kofi"))]
async fn an_unlinked_email_is_recorded_but_unattributed(pool: PgPool) {
    let service = entitlements(&pool);
    let paid = Payment {
        message_id: "msg-6",
        email_hash: "hash-of-a-stranger",
        tier_name: Some("Hosting £2.50"),
        amount_cents: 250,
        currency: "GBP",
        message: None,
    };

    let outcome = settle(&pool, &service, &paid).await.ok();

    assert!(matches!(outcome, Some(Settlement::Unmatched(_))));
}

#[sqlx::test(migrations = "../../migrations", fixtures("kofi"))]
async fn an_off_ladder_amount_is_never_attributed(pool: PgPool) {
    let service = entitlements(&pool);
    let paid = Payment {
        message_id: "msg-7",
        email_hash: "hash-of-payer",
        tier_name: Some("Coffee"),
        amount_cents: 300,
        currency: "GBP",
        message: None,
    };

    let outcome = settle(&pool, &service, &paid).await.ok();

    assert!(matches!(outcome, Some(Settlement::Unmatched(_))));
}

#[sqlx::test(migrations = "../../migrations", fixtures("kofi"))]
async fn settling_clears_the_suspension(pool: PgPool) {
    let service = entitlements(&pool);
    let paid = payment("msg-8", "Hosting £2.50", Some("BCDFGH"));

    settle(&pool, &service, &paid).await.ok();

    let row = store::get(&pool, 1).await.ok();
    let state = row.map(|r| r.state);

    assert_eq!(state.as_deref(), Some("active"));
}
