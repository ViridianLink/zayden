use std::sync::Arc;

use serenity::all::{Context, PartialGuildThread};
use sqlx::PgPool;
use zayden_app::state::AppState;

use super::Handler;
use crate::Result;
use crate::bindings::ticket;

impl Handler {
    pub async fn thread_delete(
        ctx: &Context,
        thread: &PartialGuildThread,
        pool: &PgPool,
        app: &Arc<AppState>,
    ) -> Result<()> {
        lfg::events::thread_delete(&ctx.http, thread, pool).await?;
        ticket::events::thread_delete(thread, app).await?;

        Ok(())
    }
}
