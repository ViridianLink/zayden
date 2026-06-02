use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Form, Json};
use serde::Deserialize;
use sha2::{Digest, Sha256};
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
use crate::middleware::auth::AuthUser;

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

#[derive(Deserialize)]
pub(super) struct KoFiLinkBody {
    email: String,
}

/// POST /kofi/link
///
/// Links the authenticated Discord user's account to a Ko-fi email address.
/// Stores `sha256(lowercase(email))` so plain-text addresses are never persisted.
pub(super) async fn kofi_link_handler(
    Extension(user): Extension<AuthUser>,
    State(state): State<WebState>,
    Json(body): Json<KoFiLinkBody>,
) -> Response {
    let email_hash = {
        let digest = Sha256::digest(body.email.to_lowercase());
        digest.iter().fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
    };

    let Ok(discord_user_id) = user.id.parse::<i64>() else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    match sqlx::query(
        "INSERT INTO kofi_links (email_hash, discord_user_id) VALUES ($1, $2)",
    )
    .bind(&email_hash)
    .bind(discord_user_id)
    .execute(&state.app.db)
    .await
    {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(sqlx::Error::Database(e))
            if e.constraint() == Some("kofi_links_email_hash_key") =>
        {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "This Ko-fi email is already linked to another account"})),
            )
                .into_response()
        },
        Err(e) => {
            warn!(?e, "failed to insert kofi_links row");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        },
    }
}
