use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Form, Json};
use jiff::Timestamp;
use serde::Deserialize;
use sha2::{Digest, Sha256};
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

use crate::WebState;
use crate::middleware::auth::AuthUser;

#[derive(Deserialize)]
pub(super) struct KoFiForm {
    data: String,
}

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

    if !payload.verification_ok(state.kofi_verification_token.as_deref()) {
        warn!(
            transaction_id = %payload.kofi_transaction_id,
            "Ko-fi webhook rejected: verification_token missing, unconfigured, or mismatched"
        );
        return StatusCode::OK;
    }

    if payload.kind != KoFiType::Subscription {
        return StatusCode::OK;
    }

    let email_hash = {
        let digest = Sha256::digest(payload.email.to_lowercase());
        digest.iter().fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
    };

    let discord_user_id = match sqlx::query_scalar::<_, i64>(
        "SELECT discord_user_id FROM kofi_links WHERE email_hash = $1",
    )
    .bind(&email_hash)
    .fetch_optional(&state.app.db)
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
        if let Err(e) = KoFiProvider.grant(&state.app.entitlements, grant_data).await
        {
            warn!(?e, transaction_id = %payload.kofi_transaction_id, "failed to record Ko-fi entitlement");
        }
    } else {
        if let Err(e) =
            state.app.entitlements.revoke_all_by_scope("kofi", &scope).await
        {
            warn!(?e, transaction_id = %payload.kofi_transaction_id, "failed to revoke Ko-fi entitlement");
        }
    }

    StatusCode::OK
}

#[derive(Deserialize)]
pub(super) struct KoFiLinkBody {
    email: String,
}

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

    match sqlx::query!(
        "INSERT INTO kofi_links (email_hash, discord_user_id) VALUES ($1, $2)", &email_hash, discord_user_id
    )
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
