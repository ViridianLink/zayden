use serenity::all::{
    ChannelId,
    ChannelType,
    EditThread,
    ForumTagId,
    GuildId,
    Http,
    InteractionGuildThread,
    ThreadId,
};

use crate::Result;

pub const SOLVED: &str = "[Solved] - ";
pub const CLOSED: &str = "[Closed] - ";
const PREFIXES: [&str; 3] = [SOLVED, CLOSED, "[Fixed] - "];

pub async fn mark(
    http: &Http,
    guild_id: GuildId,
    support_channel_id: ChannelId,
    thread: &InteractionGuildThread,
    tag: Option<ForumTagId>,
    prefix: &str,
) -> Result<()> {
    match usable_tag(http, guild_id, support_channel_id, tag).await? {
        Some(tag) => add_tag(http, guild_id, thread.id, tag).await,
        None => rename(http, thread, prefix).await,
    }
}

pub async fn clear(
    http: &Http,
    guild_id: GuildId,
    support_channel_id: ChannelId,
    thread: &InteractionGuildThread,
    tags: &[Option<ForumTagId>],
) -> Result<()> {
    let mut configured = Vec::new();

    for tag in tags {
        if let Some(tag) =
            usable_tag(http, guild_id, support_channel_id, *tag).await?
        {
            configured.push(tag);
        }
    }

    if configured.is_empty() {
        return rename(http, thread, "").await;
    }

    remove_tags(http, guild_id, thread.id, &configured).await
}

async fn usable_tag(
    http: &Http,
    guild_id: GuildId,
    support_channel_id: ChannelId,
    tag: Option<ForumTagId>,
) -> Result<Option<ForumTagId>> {
    let Some(tag) = tag else {
        return Ok(None);
    };

    let parent = support_channel_id.to_guild_channel(http, Some(guild_id)).await?;

    if parent.base.kind != ChannelType::Forum {
        return Ok(None);
    }

    Ok(parent.available_tags.iter().any(|t| t.id == tag).then_some(tag))
}

async fn add_tag(
    http: &Http,
    guild_id: GuildId,
    thread_id: ThreadId,
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

async fn remove_tags(
    http: &Http,
    guild_id: GuildId,
    thread_id: ThreadId,
    tags: &[ForumTagId],
) -> Result<()> {
    let thread = thread_id.to_thread(http, Some(guild_id)).await?;

    if !thread.applied_tags.iter().any(|applied| tags.contains(applied)) {
        return Ok(());
    }

    let kept = thread
        .applied_tags
        .iter()
        .copied()
        .filter(|applied| !tags.contains(applied))
        .collect::<Vec<_>>();

    thread_id.edit(http, EditThread::new().applied_tags(kept)).await?;

    Ok(())
}

async fn rename(
    http: &Http,
    thread: &InteractionGuildThread,
    prefix: &str,
) -> Result<()> {
    let name = thread.base.name.as_deref().unwrap_or_default();

    thread.id.edit(http, EditThread::new().name(retitle(name, prefix))).await?;

    Ok(())
}

#[must_use]
pub fn retitle(name: &str, prefix: &str) -> String {
    let bare = PREFIXES.iter().find_map(|p| name.strip_prefix(p)).unwrap_or(name);

    format!("{prefix}{bare}").chars().take(100).collect()
}
