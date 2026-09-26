use serenity::all::{Context, UnavailableGuild};
use sqlx::PgPool;
use tracing::{info, warn};
use zayden_app::guilds::GuildPresence;
use zayden_core::as_i64;

use super::Handler;
use crate::Result;

impl Handler {
    pub async fn guild_delete(
        ctx: &Context,
        guild: &UnavailableGuild,
        pool: &PgPool,
    ) -> Result<()> {
        if guild.unavailable {
            return Ok(());
        }

        let Some(application_id) = ctx.http.application_id() else {
            warn!(guild_id = %guild.id, "guild removal seen before the application id");
            return Ok(());
        };

        GuildPresence::left(
            pool,
            as_i64(guild.id.get()),
            as_i64(application_id.get()),
        )
        .await?;
        info!(guild_id = %guild.id, "Removed from guild; its data is kept for the retention window");

        Ok(())
    }
}
