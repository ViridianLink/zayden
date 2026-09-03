use std::sync::Arc;

use serenity::all::{Context, GuildThread};
use zayden_app::state::AppState;

use super::Handler;
use crate::Result;

impl Handler {
    pub async fn thread_create(
        ctx: &Context,
        thread: &GuildThread,
        newly_created: Option<bool>,
        app: &Arc<AppState>,
    ) -> Result<()> {
        crate::bindings::ticket::events::thread_create(
            &ctx.http,
            thread,
            newly_created,
            app,
        )
        .await?;

        Ok(())
    }
}
