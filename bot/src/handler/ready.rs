use serenity::all::{Context, OnlineStatus, Ready};
use sqlx::PgPool;

use crate::Result;
use crate::cron::start_cron_jobs;
use crate::handler::Handler;

impl Handler {
    pub async fn ready(ctx: &Context, ready: &Ready, pool: &PgPool) -> Result<()> {
        println!("{} is connected!", ready.user.name);

        ctx.set_presence(None, OnlineStatus::Online);

        let ctx = ctx.clone();
        let pool = pool.clone();

        tokio::spawn(async move { start_cron_jobs(ctx, pool).await });

        Ok(())
    }
}
