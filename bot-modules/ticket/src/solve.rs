use std::sync::Arc;
use std::time::Duration;

use serenity::all::{
    ChannelId,
    EditThread,
    GuildId,
    Http,
    HttpError,
    InteractionGuildThread,
    JsonErrorCode,
    ThreadId,
};
use tokio::time::sleep;
use tracing::warn;
use zayden_app::config::ARCHIVE_NEVER;
use zayden_app::state::AppState;

use crate::faq::on_ticket_solved;
use crate::idle::ThreadActivity;
use crate::{Result, TicketGuildRow, TicketStores, state};

pub(crate) async fn mark_solved(
    http: &Arc<Http>,
    app: &Arc<AppState>,
    stores: TicketStores<'_>,
    guild_id: GuildId,
    row: &TicketGuildRow,
    support_channel_id: ChannelId,
    thread: &InteractionGuildThread,
) -> Result<()> {
    state::mark(
        http,
        guild_id,
        support_channel_id,
        thread,
        row.solved_tag_id(),
        state::SOLVED,
    )
    .await?;

    ThreadActivity::pause(&app.db, thread.id).await?;

    schedule_archive(Arc::clone(http), thread.id, row.solved_archive_secs);

    on_ticket_solved(http, app, stores, thread.id, guild_id).await;

    Ok(())
}

fn schedule_archive(http: Arc<Http>, thread_id: ThreadId, secs: i32) {
    if secs == ARCHIVE_NEVER {
        return;
    }

    let delay = Duration::from_secs(u64::try_from(secs).unwrap_or_default());

    tokio::spawn(async move {
        sleep(delay).await;

        match thread_id.edit(&http, EditThread::new().archived(true)).await {
            Ok(_) => {},
            // The thread can be deleted while the archive is pending.
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(resp)))
                if resp.error.code == JsonErrorCode::UnknownChannel => {},
            Err(e) => warn!(?thread_id, "failed to archive solved thread: {e}"),
        }
    });
}
