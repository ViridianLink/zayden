use serenity::all::{
    ChannelId,
    ChannelType,
    EditThread,
    ForumTagId,
    GuildId,
    Http,
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
    thread_id: ThreadId,
    tag: Option<ForumTagId>,
    prefix: &str,
) -> Result<()> {
    let tag = usable_tag(http, guild_id, support_channel_id, tag).await?;

    let Some(edit) = marking(http, guild_id, thread_id, tag, prefix).await? else {
        return Ok(());
    };

    thread_id.edit(http, edit).await?;

    Ok(())
}

pub(crate) async fn marking(
    http: &Http,
    guild_id: GuildId,
    thread_id: ThreadId,
    tag: Option<ForumTagId>,
    prefix: &str,
) -> Result<Option<EditThread<'static>>> {
    let thread = thread_id.to_thread(http, Some(guild_id)).await?;

    let Some(tag) = tag else {
        return Ok(Some(EditThread::new().name(retitle(&thread.base.name, prefix))));
    };

    if thread.applied_tags.contains(&tag) {
        return Ok(None);
    }

    let mut tags = thread.applied_tags.to_vec();
    tags.insert(0, tag);

    Ok(Some(EditThread::new().applied_tags(tags)))
}

pub async fn clear(
    http: &Http,
    guild_id: GuildId,
    support_channel_id: ChannelId,
    thread_id: ThreadId,
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

    let thread = thread_id.to_thread(http, Some(guild_id)).await?;

    if configured.is_empty() {
        thread_id
            .edit(http, EditThread::new().name(retitle(&thread.base.name, "")))
            .await?;

        return Ok(());
    }

    if !thread.applied_tags.iter().any(|applied| configured.contains(applied)) {
        return Ok(());
    }

    let kept = thread
        .applied_tags
        .iter()
        .copied()
        .filter(|applied| !configured.contains(applied))
        .collect::<Vec<_>>();

    thread_id.edit(http, EditThread::new().applied_tags(kept)).await?;

    Ok(())
}

pub(crate) async fn usable_tag(
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

#[must_use]
pub fn retitle(name: &str, prefix: &str) -> String {
    let bare = PREFIXES.iter().find_map(|p| name.strip_prefix(p)).unwrap_or(name);

    format!("{prefix}{bare}").chars().take(100).collect()
}
