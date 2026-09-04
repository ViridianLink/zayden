use std::sync::Arc;

use serenity::all::{GuildThread, Http, Message, PartialGuildThread};
use ticket::SupportThreadCreate;
use ticket::idle::events as idle;
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

pub async fn thread_delete(
    thread: &PartialGuildThread,
    app: &Arc<AppState>,
) -> Result<()> {
    idle::thread_delete(thread, app).await?;

    Ok(())
}

pub async fn message(msg: &Message, app: &Arc<AppState>) -> Result<()> {
    idle::message(msg, app).await?;

    Ok(())
}
