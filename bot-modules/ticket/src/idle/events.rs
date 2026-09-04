use std::sync::Arc;

use serenity::all::{Message, PartialGuildThread};
use zayden_app::state::AppState;

use crate::Result;
use crate::idle::ThreadActivity;

pub async fn message(msg: &Message, app: &Arc<AppState>) -> Result<()> {
    let roles =
        msg.member.as_ref().map(|member| member.roles.to_vec()).unwrap_or_default();

    ThreadActivity::track(
        &app.db,
        msg.channel_id.expect_thread(),
        msg.author.id,
        &roles,
    )
    .await?;

    Ok(())
}

pub async fn thread_delete(
    thread: &PartialGuildThread,
    app: &Arc<AppState>,
) -> Result<()> {
    ThreadActivity::delete(&app.db, thread.id).await?;

    Ok(())
}
