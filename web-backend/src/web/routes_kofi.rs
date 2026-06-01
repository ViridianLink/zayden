use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use tracing::warn;
use zayden_app::entitlement::{
    EntitlementProvider,
    EntitlementScope,
    GrantData,
    KoFiPayload,
    KoFiProvider,
    Tier,
};

use crate::WebState;

/// Ko-fi sends a `application/x-www-form-urlencoded` body with a single `data`
/// field that is a URL-encoded JSON string.
#[derive(Deserialize)]
pub(super) struct KoFiForm {
    data: String,
}

/// POST /webhooks/kofi
///
/// Handles Ko-fi donation and subscription webhook notifications.
/// The endpoint always returns 200 so Ko-fi doesn't retry on transient DB errors.
pub(super) async fn kofi_webhook_handler(
    State(state): State<WebState>,
    Form(form): Form<KoFiForm>,
) -> impl IntoResponse {
    let payload: KoFiPayload = match serde_json::from_str(&form.data) {
        Ok(p) => p,
        Err(e) => {
            warn!(?e, "failed to parse Ko-fi webhook payload");
            return StatusCode::OK;
        },
    };

    // Only record subscription payments; donations are one-offs without a recurring
    // tier.
    if !payload.is_subscription_payment && payload.kind != "Subscription" {
        return StatusCode::OK;
    }

    // Ko-fi does not send Discord IDs directly. Map via email → user lookup is a
    // future M5 concern. For now we store the email as scope_id using a stable hash
    // so the row can be matched later when the Discord<->KoFi link is established.
    let scope_id = stable_u64_hash(&payload.email);
    let scope = EntitlementScope::User(scope_id);

    let grant_data = GrantData {
        external_id: payload.kofi_transaction_id.clone(),
        scope,
        tier: Tier::Pro,
        expires_at: None, // Ko-fi webhooks fire per-payment; no explicit end date
    };

    if let Err(e) = KoFiProvider.grant(&state.app.entitlements, grant_data).await {
        warn!(?e, transaction_id = %payload.kofi_transaction_id, "failed to record Ko-fi entitlement");
    }

    StatusCode::OK
}

/// Produce a stable `u64` from a string via FNV-1a. Used as a placeholder
/// `scope_id` for Ko-fi subscribers until their Discord account is linked.
fn stable_u64_hash(s: &str) -> u64 {
    const OFFSET: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;
    s.bytes().fold(OFFSET, |acc, b| acc.wrapping_mul(PRIME) ^ (u64::from(b)))
}
