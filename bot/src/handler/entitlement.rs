use serenity::all::{Context, Entitlement};
use tracing::warn;
use zayden_app::entitlement::{DiscordProvider, EntitlementProvider};

use crate::{BotState, Error, Result};

pub async fn entitlement_create(
    _ctx: &Context,
    entitlement: &Entitlement,
    bot_state: &BotState,
) -> Result<()> {
    let Some(grant_data) = DiscordProvider::build_grant(
        entitlement.id.get(),
        entitlement.user_id.map(|id| id.get()),
        entitlement.guild_id.map(|id| id.get()),
        entitlement.ends_at.map(|t| t.unix_timestamp()),
    ) else {
        return Ok(());
    };

    DiscordProvider
        .grant(&bot_state.app.entitlements, grant_data)
        .await
        .map_err(|e| {
            warn!(?e, "failed to record Discord entitlement");
            Error::from(e)
        })
}

pub async fn entitlement_update(
    ctx: &Context,
    entitlement: &Entitlement,
    bot_state: &BotState,
) -> Result<()> {
    entitlement_create(ctx, entitlement, bot_state).await
}

pub async fn entitlement_delete(
    _ctx: &Context,
    entitlement: &Entitlement,
    bot_state: &BotState,
) -> Result<()> {
    DiscordProvider
        .revoke(
            &bot_state.app.entitlements,
            &entitlement.id.get().to_string(),
        )
        .await
        .map_err(|e| {
            warn!(?e, "failed to revoke Discord entitlement");
            Error::from(e)
        })
}
