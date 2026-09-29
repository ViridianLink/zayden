use serenity::all::PartialGuild;
use sqlx::PgPool;
use zayden_app::guilds::GuildOwner;
use zayden_core::as_i64;

use super::Handler;
use crate::Result;

impl Handler {
    pub async fn guild_update(guild: &PartialGuild, pool: &PgPool) -> Result<()> {
        GuildOwner::record(
            pool,
            as_i64(guild.id.get()),
            as_i64(guild.owner_id.get()),
        )
        .await?;

        Ok(())
    }
}
