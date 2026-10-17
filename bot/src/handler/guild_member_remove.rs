use serenity::all::{GuildId, User};
use sqlx::PgPool;
use tracing::debug;
use zayden_core::as_i64;

use super::Handler;
use crate::Result;

impl Handler {
    pub async fn guild_member_remove(
        guild_id: GuildId,
        user: &User,
        pool: &PgPool,
    ) -> Result<()> {
        let removed = deadlock::db::guild_members::remove(
            pool,
            as_i64(guild_id.get()),
            as_i64(user.id.get()),
        )
        .await?;

        if removed {
            debug!(%guild_id, user_id = %user.id, "Reset Deadlock opt-ins after the member left");
        }

        Ok(())
    }
}
