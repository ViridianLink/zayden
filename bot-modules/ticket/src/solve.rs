use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;
use serenity::all::{ChannelId, GuildId, Http, ThreadId};
use zayden_app::state::AppState;

use crate::archive::{archive_in, notice};
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
    thread_id: ThreadId,
) -> Result<Option<i64>> {
    state::mark(
        http,
        guild_id,
        support_channel_id,
        thread_id,
        row.solved_tag_id(),
        state::SOLVED,
    )
    .await?;

    ThreadActivity::pause(&app.db, thread_id).await?;

    let deadline = notice::deadline(Timestamp::now(), row.solved_archive_secs);

    if deadline.is_some() {
        let secs = u64::try_from(row.solved_archive_secs).unwrap_or_default();

        archive_in(Arc::clone(http), thread_id, Duration::from_secs(secs));
    }

    on_ticket_solved(http, app, stores, thread_id, guild_id).await;

    Ok(deadline)
}
