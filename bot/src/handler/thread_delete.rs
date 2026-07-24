use serenity::all::{Context, PartialGuildThread};
use sqlx::PgPool;

use super::Handler;
use crate::Result;

impl Handler {
    pub async fn thread_delete(
        ctx: &Context,
        thread: &PartialGuildThread,
        pool: &PgPool,
    ) -> Result<()> {
        lfg::events::thread_delete(&ctx.http, thread, pool).await?;

        Ok(())
    }
}
