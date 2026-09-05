use std::sync::Arc;
use std::time::Duration;

use serenity::all::{
    GetMessages,
    GuildId,
    GuildThread,
    Http,
    Message,
    MessageId,
    UserId,
};
use tokio::time::sleep;
use tracing::{debug, warn};
use zayden_app::state::AppState;

use crate::faq::{FaqContext, TicketOpening, on_ticket_opened};
use crate::idle::ThreadActivity;
use crate::{ISSUE_EMBED_TITLE, Result, TicketGuildRow, TicketStores};

const OPENING_ATTEMPTS: u32 = 6;
const OPENING_BACKOFF: Duration = Duration::from_millis(500);
const OPENING_LIMIT: u8 = 5;
const OLDEST: MessageId = MessageId::new(1);

pub struct SupportThreadCreate;

impl SupportThreadCreate {
    pub async fn run(
        http: &Arc<Http>,
        thread: &GuildThread,
        newly_created: Option<bool>,
        app: &Arc<AppState>,
    ) -> Result<()> {
        if newly_created != Some(true) {
            return Ok(());
        }

        let guild_id = thread.base.guild_id;
        let stores = TicketStores::from_app(app);

        let Some(row) = TicketGuildRow::get(stores, &app.db, guild_id).await? else {
            debug!(%guild_id, "no ticket configuration for guild; ignoring thread");
            return Ok(());
        };

        if row.channel_id() != Some(thread.parent_id) {
            debug!(
                %guild_id,
                thread_id = %thread.id,
                "thread is not in the support channel; ignoring",
            );
            return Ok(());
        }

        ThreadActivity::insert(&app.db, guild_id, thread.id, thread.owner_id)
            .await?;

        let context = match FaqContext::load(stores.faq, guild_id).await {
            Ok(Some(context)) => context,
            Ok(None) => return Ok(()),
            Err(e) => {
                warn!(error = ?e, %guild_id, "could not load faq settings");
                return Ok(());
            },
        };

        if !context.auto_triage {
            return Ok(());
        }

        let Some((author, content)) = read_opening(http, thread).await else {
            warn!(
                thread_id = %thread.id,
                "support thread opened without a readable message; skipping triage",
            );
            return Ok(());
        };

        on_ticket_opened(
            Arc::clone(http),
            Arc::clone(app),
            context,
            TicketOpening {
                thread_id: thread.id,
                guild_id,
                author,
                title: thread.base.name.to_string(),
                tags: tag_names(http, guild_id, thread).await,
                content,
            },
        );

        Ok(())
    }
}

async fn tag_names(
    http: &Http,
    guild_id: GuildId,
    thread: &GuildThread,
) -> Vec<String> {
    if thread.applied_tags.is_empty() {
        return Vec::new();
    }

    let parent = match thread.parent_id.to_guild_channel(http, Some(guild_id)).await
    {
        Ok(parent) => parent,
        Err(e) => {
            warn!(
                error = ?e,
                channel_id = %thread.parent_id,
                "could not read the support channel's forum tags",
            );
            return Vec::new();
        },
    };

    thread
        .applied_tags
        .iter()
        .filter_map(|id| parent.available_tags.iter().find(|tag| tag.id == *id))
        .map(|tag| tag.name.to_string())
        .collect()
}

async fn read_opening(
    http: &Http,
    thread: &GuildThread,
) -> Option<(UserId, String)> {
    for attempt in 0..OPENING_ATTEMPTS {
        if attempt > 0 {
            sleep(OPENING_BACKOFF).await;
        }

        let mut messages = match thread
            .id
            .widen()
            .messages(http, GetMessages::new().after(OLDEST).limit(OPENING_LIMIT))
            .await
        {
            Ok(messages) => messages,
            Err(e) => {
                warn!(error = ?e, thread_id = %thread.id, "could not read support thread");
                continue;
            },
        };

        // Discord pages newest-first even when reading forwards, and the ticket
        // is the oldest message, not the newest.
        messages.sort_unstable_by_key(|message| message.id);

        if let Some(opening) = messages.iter().find_map(issue) {
            return Some(opening);
        }
    }

    None
}

fn issue(message: &Message) -> Option<(UserId, String)> {
    let embed = message.embeds.iter().find_map(|embed| {
        if embed.title.as_deref() != Some(ISSUE_EMBED_TITLE) {
            return None;
        }

        embed.description.as_deref().map(str::trim).filter(|d| !d.is_empty())
    });

    opening(message.author.bot(), message.author.id, &message.content, embed)
}

#[must_use]
pub fn opening(
    from_bot: bool,
    author_id: UserId,
    content: &str,
    issue_embed: Option<&str>,
) -> Option<(UserId, String)> {
    if let Some(issue) = issue_embed {
        return Some((author(content)?, issue.to_owned()));
    }

    if from_bot {
        return None;
    }

    let content = content.trim();

    (!content.is_empty()).then(|| (author_id, content.to_owned()))
}

#[must_use]
pub fn author(content: &str) -> Option<UserId> {
    let mut rest = content;

    while let Some(open) = rest.find("<@") {
        let after = rest.get(open + 2..)?;
        let close = after.find('>')?;
        let (raw, tail) = after.split_at(close);
        rest = tail;

        if raw.starts_with('&') {
            continue;
        }

        // `<@!id>` is the legacy nickname mention form.
        if let Ok(id) = raw.trim_start_matches('!').parse::<u64>()
            && id != u64::MAX
        {
            return Some(UserId::new(id));
        }
    }

    None
}
