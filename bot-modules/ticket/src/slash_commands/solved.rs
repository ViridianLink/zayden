use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;
use serenity::all::{
    ChannelType,
    CommandInteraction,
    CreateInteractionResponseFollowup,
    EditInteractionResponse,
    EditThread,
    ForumTagId,
    GenericInteractionChannel,
    GuildId,
    Http,
    HttpError,
    JsonErrorCode,
    ThreadId,
};
use sqlx::PgPool;
use tokio::time::sleep;
use tracing::warn;
use zayden_app::config::ARCHIVE_NEVER;

use crate::{Result, Ticket, TicketError, TicketGuildRow, TicketStores, donation};

impl Ticket {
    pub(super) async fn solved(
        http: &Arc<Http>,
        interaction: &CommandInteraction,
        stores: TicketStores<'_>,
        pool: &PgPool,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let row = TicketGuildRow::get(stores, pool, guild_id)
            .await?
            .ok_or(TicketError::NotInSupportChannel)?;
        let support_channel_id =
            row.channel_id().ok_or(TicketError::NotInSupportChannel)?;

        let GenericInteractionChannel::Thread(thread) = &interaction.channel else {
            return Err(TicketError::NotInSupportChannel);
        };

        if thread.parent_id != support_channel_id {
            return Err(TicketError::NotInSupportChannel);
        }

        let parent =
            support_channel_id.to_guild_channel(http, Some(guild_id)).await?;

        if parent.base.kind == ChannelType::Forum {
            let tag =
                row.solved_tag_id().ok_or(TicketError::SolvedTagNotConfigured)?;

            if !parent.available_tags.iter().any(|t| t.id == tag) {
                return Err(TicketError::SolvedTagMissing);
            }

            apply_solved_tag(http, thread.id, guild_id, tag).await?;
        } else {
            let name: String = format!(
                "[Solved] - {}",
                thread.base.name.as_deref().unwrap_or_default()
            )
            .chars()
            .take(100)
            .collect();

            thread.id.edit(http, EditThread::new().name(name)).await?;
        }

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

        Ok(())
    }
}

async fn apply_solved_tag(
    http: &Http,
    thread_id: ThreadId,
    guild_id: GuildId,
    tag: ForumTagId,
) -> Result<()> {
    let thread = thread_id.to_thread(http, Some(guild_id)).await?;

    if thread.applied_tags.contains(&tag) {
        return Ok(());
    }

    let mut tags = thread.applied_tags.to_vec();
    tags.insert(0, tag);

    thread_id.edit(http, EditThread::new().applied_tags(tags)).await?;

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
