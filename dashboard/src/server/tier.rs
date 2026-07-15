use leptos::prelude::*;

use crate::dto::UserTierInfo;

#[server]
pub async fn get_user_tier() -> Result<UserTierInfo, ServerFnError> {
    use std::sync::Arc;

    use sqlx::{PgPool, Row};
    use tower_cookies::Cookies;
    use zayden_app::state::AppState;

    use crate::app::UpgradeUrl;
    use crate::dto::Tier;
    use crate::server::auth::server_err;

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };
    let upgrade_url = use_context::<UpgradeUrl>().and_then(|u| u.0);

    let cookies: Cookies = leptos_axum::extract().await.map_err(server_err)?;

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Ok(UserTierInfo { tier: None, upgrade_url });
    };

    let row = sqlx::query(
        "SELECT discord_user_id FROM web_sessions WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(server_err)?;

    let Some(row) = row else {
        return Ok(UserTierInfo { tier: None, upgrade_url });
    };
    let user_id = row.get::<i64, _>("discord_user_id").cast_unsigned();

    let tier = app.entitlements.user_tier(user_id).await;
    Ok(UserTierInfo { tier: Tier::from_key(tier.as_str()), upgrade_url })
}
