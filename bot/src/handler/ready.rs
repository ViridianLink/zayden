use std::num::NonZeroU16;
use std::sync::Arc;

use serenity::all::{Context, OnlineStatus, Ready};
use tracing::{info, warn};
use zayden_app::guilds::{GuildPresence, Shard};
use zayden_core::as_i64;

use crate::bindings::ai::Ai;
use crate::cron::start_cron_jobs;
use crate::handler::Handler;
use crate::patreon_webhook::spawn_patreon_listener;
use crate::youtube_webhook::spawn_youtube_listener;
use crate::{BotState, Result, module_sync};

impl Handler {
    pub async fn ready(&self, ctx: &Context, ready: &Ready) -> Result<()> {
        info!(
            "{} is connected ({} shards) and in {} guilds!",
            ready.user.name,
            ready.shard.map_or(NonZeroU16::MIN, |info| info.total),
            ready.guilds.len()
        );

        Ai::identify(&ready.user);

        ctx.set_presence(None, OnlineStatus::Online);

        let pool = self.app.db.clone();
        BotState::ready(ctx, ready, &pool, self.app.zayden_id).await?;
        Self::reconcile_presence(ready, &pool).await;

        if self.cron_started.set(()).is_ok() {
            self.background_tasks(ctx, ready).await;
        }

        Ok(())
    }

    async fn reconcile_presence(ready: &Ready, pool: &sqlx::PgPool) {
        let shard = ready.shard.map_or(Shard { id: 0, total: 1 }, |info| Shard {
            id: info.id.0,
            total: info.total.get(),
        });
        let present = ready
            .guilds
            .iter()
            .map(|guild| as_i64(guild.id.get()))
            .collect::<Vec<_>>();

        match GuildPresence::reconcile(
            pool,
            as_i64(ready.application.id.get()),
            shard,
            &present,
        )
        .await
        {
            Ok(0) => {},
            Ok(marked) => {
                info!(marked, "guilds removed while offline enter retention");
            },
            Err(e) => warn!(error = ?e, "failed to reconcile guild presence"),
        }
    }

    async fn background_tasks(&self, ctx: &Context, ready: &Ready) {
        if ready.application.id.get() == self.app.zayden_id {
            self.bot_state.write().await.setup_static_cron();
        }

        let palworld = Arc::clone(&self.bot_state.read().await.palworld);
        tokio::spawn(async move { palworld.warm().await });

        spawn_patreon_listener(ctx.clone(), Arc::clone(&self.app));
        let youtube = self.bot_state.read().await.youtube.clone();
        if let Some(runtime) = youtube {
            spawn_youtube_listener(ctx.clone(), Arc::clone(&self.app), runtime);
        }
        module_sync::spawn_listener(
            Arc::clone(&ctx.http),
            Arc::clone(&self.app),
            Arc::clone(&self.registry),
        );

        let http = Arc::clone(&ctx.http);
        let app = Arc::clone(&self.app);
        let guilds = ready.guilds.iter().map(|guild| guild.id).collect::<Vec<_>>();
        tokio::spawn(async move {
            ticket::archive::rebuild(&http, &app, &guilds).await;
        });

        let ctx = ctx.clone();
        let pool = self.app.db.clone();
        tokio::spawn(async move { start_cron_jobs(ctx, pool).await });
    }
}
