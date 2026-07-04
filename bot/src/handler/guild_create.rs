use serenity::all::{Context, Guild};
use sqlx::{PgPool, Postgres};
use tokio::sync::RwLock;
use tracing::info;

use lfg::GuildTable;

use super::Handler;
use crate::bindings::lfg::PostTable;
use crate::{BotState, Result};

impl Handler {
    pub async fn guild_create(
        &self,
        ctx: &Context,
        guild: &Guild,
        pool: &PgPool,
    ) -> Result<()> {
        let data = ctx.data::<RwLock<BotState>>();

        let (lfg_result, ()) = tokio::join!(
            lfg::events::guild_create::<BotState, Postgres, GuildTable, PostTable>(
                ctx, guild, pool
            ),
            BotState::guild_create(data, guild),
        );
        lfg_result?;

        let commands = self.registry.definitions_for(guild.id);

        guild.id.set_commands(&ctx.http, &commands).await?;
        info!("Registered {}", guild.name);

        Ok(())
    }
}
