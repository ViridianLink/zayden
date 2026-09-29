use std::sync::Arc;

use jiff::Timestamp;
use serenity::all::{Context, Guild};
use sqlx::PgPool;
use tokio::sync::RwLock;
use tracing::{info, warn};
use zayden_app::guilds::{GuildOwner, GuildPresence, JoinKind};
use zayden_core::as_i64;

use super::Handler;
use crate::{BotState, Result, module_sync};

impl Handler {
    pub async fn guild_create(
        &self,
        ctx: &Context,
        guild: &Guild,
        pool: &PgPool,
    ) -> Result<()> {
        let joined_at =
            Timestamp::from_millisecond(guild.joined_at.unix_timestamp_millis())?;

        let join = match ctx.http.application_id() {
            Some(application_id) => GuildPresence::joined(
                pool,
                as_i64(guild.id.get()),
                as_i64(application_id.get()),
                joined_at,
            )
            .await
            .unwrap_or_else(|e| {
                warn!(error = ?e, guild_id = %guild.id, "failed to record guild presence");
                JoinKind::Reconnect
            }),
            None => {
                warn!(guild_id = %guild.id, "guild seen before the application id");
                JoinKind::Reconnect
            },
        };

        if let Err(e) = GuildOwner::record(
            pool,
            as_i64(guild.id.get()),
            as_i64(guild.owner_id.get()),
        )
        .await
        {
            warn!(error = ?e, guild_id = %guild.id, "failed to record guild owner");
        }

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

        let states = module_sync::seed(
            &ctx.http,
            &self.app,
            &self.registry,
            guild,
            joined_at,
            join,
        )
        .await?;

        module_sync::sync_commands(&ctx.http, &self.registry, guild.id, &states)
            .await?;
        info!("Registered {}", guild.name);

        Ok(())
    }
}
