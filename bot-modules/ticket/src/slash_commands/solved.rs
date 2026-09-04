use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;
use serenity::all::{
    CommandInteraction,
    CreateInteractionResponseFollowup,
    EditInteractionResponse,
    EditThread,
    GuildId,
    Http,
    HttpError,
    JsonErrorCode,
    ThreadId,
};
use tokio::time::sleep;
use tracing::warn;
use zayden_app::config::ARCHIVE_NEVER;
use zayden_app::state::AppState;

use crate::faq::on_ticket_solved;
use crate::{
    Result,
    Ticket,
    TicketError,
    TicketGuildRow,
    TicketStores,
    donation,
    state,
    support_thread,
};

impl Ticket {
    pub(super) async fn solved(
        http: &Arc<Http>,
        interaction: &CommandInteraction,
        stores: TicketStores<'_>,
        app: &Arc<AppState>,
        guild_id: GuildId,
    ) -> Result<()> {
        let pool = &app.db;

        interaction.defer(http).await?;

        let row = TicketGuildRow::get(stores, pool, guild_id)
            .await?
            .ok_or(TicketError::NotInSupportChannel)?;
        let support_channel_id =
            row.channel_id().ok_or(TicketError::NotInSupportChannel)?;

        let thread = support_thread(&interaction.channel, support_channel_id)?;

        state::mark(
            http,
            guild_id,
            support_channel_id,
            thread,
            row.solved_tag_id(),
            state::SOLVED,
        )
        .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content(format!(
                    "This post has been marked as solved.\n-# Post closed <t:{}:R>",
                    Timestamp::now().as_second()
                )),
            )
            .await?;

        if let Some(helper_role) = row.helper_role_id()
            && let Some(message) =
                donation::message(http, pool, thread.id, guild_id, helper_role)
                    .await?
        {
            interaction
                .create_followup(
                    http,
                    CreateInteractionResponseFollowup::new().content(message),
                )
                .await?;
        }

        schedule_archive(Arc::clone(http), thread.id, row.solved_archive_secs);

        on_ticket_solved(http, app, stores, thread.id, guild_id).await;

        Ok(())
    }
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
