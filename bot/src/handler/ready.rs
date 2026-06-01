use std::num::NonZeroU16;

use serenity::all::{Context, OnlineStatus, Ready};
use tracing::info;

use crate::cron::start_cron_jobs;
use crate::handler::Handler;
use crate::{BotState, Result, ZAYDEN_ID};

impl Handler {
    pub async fn ready(&self, ctx: &Context, ready: &Ready) -> Result<()> {
        info!(
            "{} is connected ({} shards) and in {} guilds!",
            ready.user.name,
            ready.shard.map_or(NonZeroU16::MIN, |info| info.total),
            ready.guilds.len()
        );

        ctx.set_presence(None, OnlineStatus::Online);

        let pool = self.app.db.clone();
        BotState::ready(ctx, ready, &pool).await;

        if self.cron_started.set(()).is_ok() {
            if ready.application.id.get() == ZAYDEN_ID.get() {
                self.bot_state.write().await.setup_static_cron();
            }

            let ctx = ctx.clone();
            let pool = self.app.db.clone();
            tokio::spawn(async move { start_cron_jobs(ctx, pool).await });
        }

        Ok(())
    }
}
