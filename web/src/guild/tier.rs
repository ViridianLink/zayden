use topcoat::context::Cx;
use twilight_model::id::Id;

use super::dto::{Tier, UserTierInfo};
use super::error::GuildError;
use crate::auth::{
    AuthError,
    app_state,
    current_session_identity,
    discord_client,
    web_state,
};

pub async fn guild_server_tier(cx: &Cx, guild_id: u64) -> Result<Tier, GuildError> {
    let app = app_state(cx)?;

    let guild = Id::new_checked(guild_id).ok_or(AuthError::InvalidGuildId)?;

    let owner = discord_client(cx)?.guild(guild).await?.model().await?.owner_id;

    let tier = app.entitlements.server_tier(guild_id, owner.get()).await;

    Ok(Tier::from_key(tier.as_str()).unwrap_or(Tier::Free))
}

pub async fn get_user_tier(cx: &Cx) -> Result<UserTierInfo, GuildError> {
    let app = app_state(cx)?;
    let upgrade_url =
        web_state(cx).ok().and_then(|state| state.urls.upgrade.clone());

    let Some(identity) = current_session_identity(cx).await? else {
        return Ok(UserTierInfo { tier: None, upgrade_url });
    };
    let user_id = identity.user_id.cast_unsigned();

    let tier = app.entitlements.user_tier(user_id).await;
    Ok(UserTierInfo { tier: Tier::from_key(tier.as_str()), upgrade_url })
}
