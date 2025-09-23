use std::num::NonZeroU16;
use std::sync::atomic::Ordering;

use futures::future;
use serenity::all::{Context, OnlineStatus, Ready};
use tokio::sync::RwLock;

use crate::cron::start_cron_jobs;
use crate::handler::Handler;
use crate::{CtxData, Result, ZAYDEN_ID, modules};

impl Handler {
    pub async fn ready(&self, ctx: &Context, ready: &Ready) -> Result<()> {
        println!(
            "{} is connected ({} shards) and in {} guilds!",
            ready.user.name,
            ready
                .shard
                .map(|info| info.total)
                .unwrap_or(NonZeroU16::MIN),
            ready.guilds.len()
        );

        ctx.set_presence(None, OnlineStatus::Online);

        CtxData::ready(ctx, ready, &self.pool).await;

        let commands = modules::global_register(ctx);
        let iter = ready
            .guilds
            .iter()
            .map(|guild| guild.id.set_commands(&ctx.http, &commands));
        future::try_join_all(iter).await.unwrap();
        println!("Updated all commands");

        if !self.started_cron.load(Ordering::Relaxed) {
            {
                let data = ctx.data::<RwLock<CtxData>>();
                let mut data = data.write().await;
                if ready.application.id.get() == ZAYDEN_ID.get() {
                    data.setup_static_cron();
                }
            }

            let ctx = ctx.clone();
            let pool = self.pool.clone();

            tokio::spawn(async move { start_cron_jobs(ctx, pool).await });

            self.started_cron.store(true, Ordering::Relaxed);
        }

        Ok(())
    }
}
