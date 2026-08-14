use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::app::UpgradeUrl,
    crate::dto::Tier,
    crate::server::auth::{app_state, db_pool, discord_client, server_err},
    tower_cookies::Cookies,
};

use crate::dto::UserTierInfo;

#[cfg(feature = "ssr")]
pub(crate) async fn guild_server_tier(guild_id: u64) -> Result<Tier, ServerFnError> {
    use twilight_model::id::Id;

    let app = app_state()?;

    let owner = discord_client()?
        .guild(Id::new(guild_id))
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?
        .owner_id;

    let tier = app.entitlements.server_tier(guild_id, owner.get()).await;

    Ok(Tier::from_key(tier.as_str()).unwrap_or(Tier::Free))
}

#[server]
pub async fn get_user_tier() -> Result<UserTierInfo, ServerFnError> {
    let pool = db_pool()?;
    let app = app_state()?;
    let upgrade_url = use_context::<UpgradeUrl>().and_then(|u| u.0);

    let cookies: Cookies = leptos_axum::extract().await.map_err(server_err)?;

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Ok(UserTierInfo { tier: None, upgrade_url });
    };

    let row = sqlx::query_scalar!(
        "SELECT discord_user_id FROM web_sessions WHERE token = $1 AND expires_at > now()",
        &token,
    )
    .fetch_optional(&pool)
    .await
    .map_err(server_err)?;

    let Some(discord_user_id) = row else {
        return Ok(UserTierInfo { tier: None, upgrade_url });
    };
    let user_id = discord_user_id.cast_unsigned();

    let tier = app.entitlements.user_tier(user_id).await;
    Ok(UserTierInfo { tier: Tier::from_key(tier.as_str()), upgrade_url })
}
