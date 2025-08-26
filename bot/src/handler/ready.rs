use std::sync::atomic::Ordering;

use serenity::all::{Context, OnlineStatus, Ready};
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::cron::start_cron_jobs;
use crate::handler::Handler;
use crate::{CtxData, Result, ZAYDEN_ID};

impl Handler {
    pub async fn ready(&self, ctx: &Context, ready: &Ready, pool: &PgPool) -> Result<()> {
        println!("{} is connected!", ready.user.name);

        ctx.set_presence(None, OnlineStatus::Online);

        CtxData::ready(ctx, ready).await;

        if !self.started_cron.load(Ordering::Relaxed) {
            {
                let data = ctx.data::<RwLock<CtxData>>();
                let mut data = data.write().await;
                if ready.application.id.get() == ZAYDEN_ID.get() {
                    data.setup_static_cron();
                }
            }

            let ctx = ctx.clone();
            let pool = pool.clone();

            tokio::spawn(async move { start_cron_jobs(ctx, pool).await });

            self.started_cron.store(true, Ordering::Relaxed);
        }

        Ok(())
    }
}
