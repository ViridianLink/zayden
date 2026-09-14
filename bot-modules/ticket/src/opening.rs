use std::time::Duration;

use serenity::all::{
    GetMessages,
    GuildId,
    GuildThread,
    Http,
    Message,
    MessageId,
    ThreadId,
    UserId,
};
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::ISSUE_EMBED_TITLE;
use crate::faq::{TicketOpening, screenshots};

const OPENING_BACKOFF: Duration = Duration::from_millis(500);
const OPENING_LIMIT: u8 = 5;
const OLDEST: MessageId = MessageId::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningMessage {
    pub author: UserId,
    pub text: String,
    pub images: Vec<String>,
}

pub(crate) async fn ticket_opening(
    http: &Http,
    thread: &GuildThread,
    attempts: u32,
) -> Option<TicketOpening> {
    let opening = read_opening(http, thread.id, attempts).await.or_else(|| {
        debug!(
            thread_id = %thread.id,
            "no readable opening message; triaging from the thread title",
        );
        title_only(thread.owner_id, bot_user(http), &thread.base.name)
    })?;

    Some(TicketOpening {
        thread_id: thread.id,
        guild_id: thread.base.guild_id,
        author: opening.author,
        title: thread.base.name.to_string(),
        tags: tag_names(http, thread.base.guild_id, thread).await,
        content: opening.text,
        images: opening.images,
    })
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

fn bot_user(http: &Http) -> Option<UserId> {
    http.application_id().map(|id| UserId::new(id.get()))
}

async fn read_opening(
    http: &Http,
    thread_id: ThreadId,
    attempts: u32,
) -> Option<OpeningMessage> {
    for attempt in 0..attempts {
        if attempt > 0 {
            sleep(OPENING_BACKOFF).await;
        }

        let mut messages = match thread_id
            .widen()
            .messages(http, GetMessages::new().after(OLDEST).limit(OPENING_LIMIT))
            .await
        {
            Ok(messages) => messages,
            Err(e) => {
                warn!(error = ?e, %thread_id, "could not read support thread");
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

fn issue(message: &Message) -> Option<OpeningMessage> {
    let embed = message.embeds.iter().find_map(|embed| {
        if embed.title.as_deref() != Some(ISSUE_EMBED_TITLE) {
            return None;
        }

        embed.description.as_deref().map(str::trim).filter(|d| !d.is_empty())
    });

    opening(
        message.author.bot(),
        message.author.id,
        &message.content,
        embed,
        screenshots::urls(&message.attachments),
    )
}

#[must_use]
pub fn opening(
    from_bot: bool,
    author_id: UserId,
    content: &str,
    issue_embed: Option<&str>,
    images: Vec<String>,
) -> Option<OpeningMessage> {
    if let Some(issue) = issue_embed {
        return Some(OpeningMessage {
            author: author(content)?,
            text: issue.to_owned(),
            images,
        });
    }

    if from_bot {
        return None;
    }

    let text = content.trim();

    (!text.is_empty() || !images.is_empty()).then(|| OpeningMessage {
        author: author_id,
        text: text.to_owned(),
        images,
    })
}

#[must_use]
pub fn title_only(
    owner_id: UserId,
    bot_id: Option<UserId>,
    title: &str,
) -> Option<OpeningMessage> {
    if title.trim().is_empty() || bot_id == Some(owner_id) {
        return None;
    }

    Some(OpeningMessage {
        author: owner_id,
        text: String::new(),
        images: Vec::new(),
    })
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
