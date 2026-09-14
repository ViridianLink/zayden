use std::sync::Arc;

use serenity::all::{Context, Guild};
use sqlx::PgPool;
use tokio::sync::RwLock;
use tracing::info;

use super::Handler;
use crate::{BotState, Result, module_sync};

impl Handler {
    pub async fn guild_create(
        &self,
        ctx: &Context,
        guild: &Guild,
        pool: &PgPool,
    ) -> Result<()> {
        let data = ctx.data::<RwLock<BotState>>();

        // Party provisioning and cleanup are lost with the in-memory cron jobs,
        // so anything the bot was down for has to be caught up here.
        let jellyfin_runtime = {
            let guard = data.read().await;
            guard.jellyfin.as_ref().map(Arc::clone)
        };

        let (lfg_result, ()) = tokio::join!(
            lfg::events::guild_create::<BotState>(ctx, guild, pool),
            BotState::guild_create(data, guild),
        );
        lfg_result?;

        if let Some(runtime) = jellyfin_runtime {
            watch::events::guild_create::<BotState>(ctx, &runtime, pool, guild.id)
                .await;
        }

        let states =
            module_sync::seed(&ctx.http, &self.app, &self.registry, guild).await?;

        module_sync::sync_commands(&ctx.http, &self.registry, guild.id, &states)
            .await?;
        info!("Registered {}", guild.name);

        Ok(())
    }
}
