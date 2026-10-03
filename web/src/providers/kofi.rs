use hosting::kofi::{Payment, Settlement};
use hosting::{kofi, pricing};
use jiff::Timestamp;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Json;
use topcoat::router::request::Bytes;
use topcoat::router::response::Response;
use topcoat::router::route;
use tracing::warn;
use zayden_app::entitlement::{
    EntitlementProvider,
    EntitlementScope,
    GrantData,
    KoFiPayload,
    KoFiProvider,
    KoFiType,
    Tier,
};
use zayden_app::state::AppState;

use super::fields;
use super::reply::{acknowledge, reject};
use crate::auth::{app_state, web_state};
use crate::util::email_hash;

#[route(POST "/webhooks/kofi")]
pub(super) async fn kofi_webhook_handler(cx: &Cx, body: Bytes) -> Result<Response> {
    let [data] = match fields::form(cx, &body, ["data"]) {
        Ok(fields) => fields,
        Err(rejection) => return reject(cx, &rejection),
    };
    let data = match fields::required_form(data, "data") {
        Ok(data) => data,
        Err(rejection) => return reject(cx, &rejection),
    };

    let app = app_state(cx)?;
    let verification_token =
        web_state(cx)?.integrations.kofi_verification_token.as_deref();

    process(app, verification_token, &data).await;
    acknowledge(cx)
}

async fn process(app: &AppState, verification_token: Option<&str>, data: &str) {
    let payload: KoFiPayload = match Json::from_bytes(data.as_bytes()) {
        Ok(Json(payload)) => payload,
        Err(e) => {
            warn!(?e, "failed to parse Ko-fi webhook payload");
            return;
        },
    };

    if !payload.verification_ok(verification_token) {
        warn!(
            transaction_id = %payload.kofi_transaction_id,
            "Ko-fi webhook rejected: verification_token missing, unconfigured, or mismatched"
        );
        return;
    }

    if payload.kind != KoFiType::Subscription {
        return;
    }

    let email_hash = email_hash(&payload.email);

    if payload.tier_name.as_deref().and_then(pricing::price_from_tier_name).is_some()
    {
        settle_hosting(app, &payload, &email_hash).await;
        return;
    }

    let discord_user_id = match sqlx::query_scalar!(
        "SELECT discord_user_id FROM kofi_links WHERE email_hash = $1",
        &email_hash,
    )
    .fetch_optional(&app.db)
    .await
    {
        Ok(Some(id)) => id.cast_unsigned(),
        Ok(None) => {
            warn!(
                transaction_id = %payload.kofi_transaction_id,
                "Ko-fi subscription event received but email is not linked to a Discord account; skipping"
            );
            return;
        },
        Err(e) => {
            warn!(?e, transaction_id = %payload.kofi_transaction_id, "failed to query kofi_links");
            return;
        },
    };

    let scope = EntitlementScope::User(discord_user_id);

    if payload.is_subscription_payment {
        let expires_at = KoFiProvider::subscription_expiry(Timestamp::now());
        let grant_data = GrantData {
            external_id: payload.kofi_transaction_id.clone(),
            scope,
            tier: Tier::Pro,
            expires_at,
        };
        if let Err(e) = KoFiProvider.grant(&app.entitlements, grant_data).await {
            warn!(?e, transaction_id = %payload.kofi_transaction_id, "failed to record Ko-fi entitlement");
        }
    } else if let Err(e) = app.entitlements.revoke_all_by_scope("kofi", &scope).await
    {
        warn!(?e, transaction_id = %payload.kofi_transaction_id, "failed to revoke Ko-fi entitlement");
    }
}

async fn settle_hosting(app: &AppState, payload: &KoFiPayload, email_hash: &str) {
    let Some(message_id) = payload.message_id.as_deref() else {
        warn!(
            transaction_id = %payload.kofi_transaction_id,
            "hosting: Ko-fi payment has no message_id; cannot deduplicate it"
        );
        return;
    };

    let amount_cents = payload
        .amount
        .as_deref()
        .and_then(pricing::parse_amount_cents)
        .unwrap_or_default();

    let payment = Payment {
        message_id,
        email_hash,
        tier_name: payload.tier_name.as_deref(),
        amount_cents,
        currency: payload.currency.as_deref().unwrap_or("GBP"),
        message: payload.message.as_deref(),
    };

    match kofi::settle(&app.db, &app.entitlements, &payment).await {
        Ok(Settlement::Settled { server_id, matched_by }) => {
            tracing::info!(server_id, matched_by, "hosting: payment settled");
        },
        Ok(Settlement::Duplicate) => {},
        Ok(Settlement::Unmatched(reason)) => {
            warn!(
                reason,
                transaction_id = %payload.kofi_transaction_id,
                "hosting: payment recorded but unattributed; reconcile by hand"
            );
        },
        Err(e) => {
            warn!(
                ?e,
                transaction_id = %payload.kofi_transaction_id,
                "hosting: failed to record payment"
            );
        },
    }
}
