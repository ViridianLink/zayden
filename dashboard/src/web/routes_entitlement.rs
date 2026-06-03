use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::{Value, json};
use zayden_app::entitlement::types::{EntitlementScope, Tier};

use crate::middleware::auth::AuthUser;
use crate::{Error, Result, WebState};

pub(super) async fn user_tier(
    Extension(user): Extension<AuthUser>,
    State(state): State<WebState>,
) -> Result<Json<Value>> {
    let Ok(user_id) = user.id.parse::<u64>() else {
        return Err(Error::BadRequest);
    };
    let tier = state.app.entitlements.user_tier(user_id).await;
    Ok(Json(json!({ "tier": tier.as_str() })))
}

pub(super) async fn guild_tier(
    Path(id): Path<String>,
    State(state): State<WebState>,
) -> Result<Json<Value>> {
    let Ok(guild_id) = id.parse::<u64>() else {
        return Err(Error::BadRequest);
    };
    let tier = state.app.entitlements.guild_tier(guild_id).await;
    Ok(Json(json!({ "tier": tier.as_str() })))
}

#[derive(Deserialize)]
pub(super) struct GrantBody {
    scope_type: String,
    scope_id: u64,
    scope_secondary_id: Option<u64>,
    tier: String,
    provider: String,
    external_id: String,
}

pub(super) async fn admin_grant(
    Extension(user): Extension<AuthUser>,
    State(state): State<WebState>,
    Json(body): Json<GrantBody>,
) -> Result<StatusCode> {
    let Ok(user_id) = user.id.parse::<u64>() else {
        return Err(Error::BadRequest);
    };
    if user_id != state.bot_owner {
        return Err(Error::Forbidden);
    }

    let scope = match body.scope_type.as_str() {
        "user" => EntitlementScope::User(body.scope_id),
        "guild" => EntitlementScope::Guild(body.scope_id),
        "user_in_guild" => EntitlementScope::UserInGuild(
            body.scope_id,
            body.scope_secondary_id.unwrap_or(0),
        ),
        _ => return Err(Error::BadRequest),
    };

    let Ok(tier) = body.tier.parse::<Tier>() else {
        return Err(Error::BadRequest);
    };

    state
        .app
        .entitlements
        .grant(scope, tier, &body.provider, &body.external_id, None)
        .await?;

    Ok(StatusCode::CREATED)
}
