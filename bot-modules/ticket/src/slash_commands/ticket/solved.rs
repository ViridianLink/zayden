use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;

use futures::{StreamExt, TryStreamExt};
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
    Mentionable,
    RoleId,
    ThreadId,
    UserId,
};
use sqlx::PgPool;
use tokio::time::sleep;
use tracing::warn;
use zayden_app::config::ARCHIVE_NEVER;

use crate::helper_links::HelperLinks;
use crate::{Result, Ticket, TicketError, TicketGuildRow, TicketStores};

/// Bounds the REST pagination cost of scanning a thread for helpers. Long
/// threads are truncated rather than paged to completion.
const HELPER_SCAN_LIMIT: usize = 500;

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
                donation_message(http, pool, thread.id, guild_id, helper_role)
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

/// Archiving is deferred rather than awaited so the interaction handler is not
/// held for the configured delay. A restart inside the window drops the
/// archive; the thread stays tagged either way.
fn schedule_archive(http: Arc<Http>, thread_id: ThreadId, secs: i32) {
    if secs == ARCHIVE_NEVER {
        return;
    }

    let delay = Duration::from_secs(u64::try_from(secs).unwrap_or_default());

    tokio::spawn(async move {
        sleep(delay).await;

        if let Err(e) = thread_id.edit(&http, EditThread::new().archived(true)).await
        {
            warn!(?thread_id, "failed to archive solved thread: {e}");
        }
    });
}

async fn donation_message(
    http: &Http,
    pool: &PgPool,
    thread_id: ThreadId,
    guild_id: GuildId,
    helper_role: RoleId,
) -> Result<Option<String>> {
    let links = HelperLinks::map(pool, guild_id).await?;

    if links.is_empty() {
        return Ok(None);
    }

    let helpers: HashMap<UserId, String> = thread_id
        .widen()
        .messages_iter(http)
        .take(HELPER_SCAN_LIMIT)
        .try_fold(HashMap::new(), async |mut helpers, m| {
            let Some(member) = m.member else { return Ok(helpers) };

            if !member.roles.contains(&helper_role) {
                return Ok(helpers);
            }

            if let Some(link) = links.get(&m.author.id) {
                helpers.insert(m.author.id, link.clone());
            }

            Ok(helpers)
        })
        .await?;

    if helpers.is_empty() {
        return Ok(None);
    }

    let mut helpers = helpers.into_iter().collect::<Vec<_>>();
    helpers.sort_unstable_by_key(|(id, _)| *id);

    let mut reply =
        String::from("If this helped, consider supporting the people who did:");

    for (id, link) in helpers {
        let _ = write!(reply, "\n{}: {link}", id.mention());
    }

    Ok(Some(reply))
}
