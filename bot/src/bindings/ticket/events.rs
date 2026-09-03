use std::sync::Arc;

use serenity::all::{GuildThread, Http};
use ticket::SupportThreadCreate;
use zayden_app::state::AppState;

use crate::Result;

pub async fn thread_create(
    http: &Arc<Http>,
    thread: &GuildThread,
    newly_created: Option<bool>,
    app: &Arc<AppState>,
) -> Result<()> {
    SupportThreadCreate::run(http, thread, newly_created, app).await?;

    Ok(())
}
