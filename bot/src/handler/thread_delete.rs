use serenity::all::{Context, PartialGuildThread};
use sqlx::{PgPool, Postgres};

use crate::Result;
use crate::modules::lfg::PostTable;

use super::Handler;

impl Handler {
    pub async fn thread_delete(
        ctx: &Context,
        thread: &PartialGuildThread,
        pool: &PgPool,
    ) -> Result<()> {
        lfg::events::thread_delete::<Postgres, PostTable>(&ctx.http, thread, pool).await;

        Ok(())
    }
}
