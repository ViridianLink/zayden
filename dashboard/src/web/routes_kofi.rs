use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Form, Json};
use dashboard::util;
use hosting::kofi::{Payment, Settlement};
use hosting::{kofi, pricing};
use jiff::Timestamp;
use serde::Deserialize;
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
use zayden_app::state::AppState as ZaydenAppState;

use crate::middleware::auth::AuthUser;
use crate::state::IntegrationsState;

#[derive(Deserialize)]
pub(super) struct KoFiForm {
    data: String,
}

pub(super) async fn kofi_webhook_handler(
    State(integrations): State<Arc<IntegrationsState>>,
    State(app): State<Arc<ZaydenAppState>>,
    Form(form): Form<KoFiForm>,
) -> impl IntoResponse {
    let payload: KoFiPayload = match serde_json::from_str(&form.data) {
        Ok(p) => p,
        Err(e) => {
            warn!(?e, "failed to parse Ko-fi webhook payload");
            return StatusCode::OK;
        },
    };

    if !payload.verification_ok(integrations.kofi_verification_token.as_deref()) {
        warn!(
            transaction_id = %payload.kofi_transaction_id,
            "Ko-fi webhook rejected: verification_token missing, unconfigured, or mismatched"
        );
        return StatusCode::OK;
    }

    if payload.kind != KoFiType::Subscription {
        return StatusCode::OK;
    }

    let email_hash = util::email_hash(&payload.email);

    // Hosting tiers are named for their ladder rung, which is what separates
    // them from the Pro/Ultra membership tiers. They must not fall through to
    // the grant below: a rented Small server would otherwise buy Pro, and Pro
    // makes Small free.
    if payload.tier_name.as_deref().and_then(pricing::price_from_tier_name).is_some()
    {
        settle_hosting(&app, &payload, &email_hash).await;
        return StatusCode::OK;
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
            return StatusCode::OK;
        },
        Err(e) => {
            warn!(?e, transaction_id = %payload.kofi_transaction_id, "failed to query kofi_links");
            return StatusCode::OK;
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

    StatusCode::OK
}

async fn settle_hosting(
    app: &ZaydenAppState,
    payload: &KoFiPayload,
    email_hash: &str,
) {
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

#[derive(Deserialize)]
pub(super) struct KoFiLinkBody {
    email: String,
}

pub(super) async fn kofi_link_handler(
    Extension(user): Extension<AuthUser>,
    State(app): State<Arc<ZaydenAppState>>,
    Json(body): Json<KoFiLinkBody>,
) -> Response {
    let email_hash = util::email_hash(&body.email);

    let Ok(discord_user_id) = user.id.parse::<i64>() else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    match sqlx::query!(
        "INSERT INTO kofi_links (email_hash, discord_user_id) VALUES ($1, $2)", &email_hash, discord_user_id
    )
    .execute(&app.db)
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
