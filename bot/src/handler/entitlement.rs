use serenity::all::{Context, Entitlement, GuildId, UserId};
use tracing::warn;
use zayden_app::entitlement::{DiscordProvider, EntitlementProvider};
use zayden_app::state::AppState;

use crate::{BotError, Result};

pub(super) async fn entitlement_create(
    _ctx: &Context,
    entitlement: &Entitlement,
    app: &AppState,
) -> Result<()> {
    let Some(grant_data) = DiscordProvider::build_grant(
        entitlement.id.get(),
        entitlement.user_id.map(UserId::get),
        entitlement.guild_id.map(GuildId::get),
        entitlement.ends_at.map(|t| t.unix_timestamp()),
    ) else {
        return Ok(());
    };

    DiscordProvider.grant(&app.entitlements, grant_data).await.map_err(|e| {
        warn!(?e, "failed to record Discord entitlement");
        BotError::from(e)
    })
}

pub(super) async fn entitlement_update(
    ctx: &Context,
    entitlement: &Entitlement,
    app: &AppState,
) -> Result<()> {
    entitlement_create(ctx, entitlement, app).await
}

pub(super) async fn entitlement_delete(
    _ctx: &Context,
    entitlement: &Entitlement,
    app: &AppState,
) -> Result<()> {
    DiscordProvider
        .revoke(&app.entitlements, &entitlement.id.get().to_string())
        .await
        .map_err(|e| {
            warn!(?e, "failed to revoke Discord entitlement");
            BotError::from(e)
        })
}
