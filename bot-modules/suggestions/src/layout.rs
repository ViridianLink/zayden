use serenity::all::{ChannelId, GenericChannelId, MessageId};

const THREAD_NAME_LIMIT: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Candidate {
    TextMessage,
    ThreadStarter,
    Ignore,
}

// A suggestion's thread id equals its starter message id in both layouts: a
// forum post's starter message shares the thread's id, and a thread created
// from a text channel message takes that message's id.
#[must_use]
pub const fn candidate(
    reaction_channel: GenericChannelId,
    message_id: MessageId,
    configured: ChannelId,
) -> Candidate {
    if reaction_channel.get() == configured.get() {
        Candidate::TextMessage
    } else if reaction_channel.get() == message_id.get() {
        Candidate::ThreadStarter
    } else {
        Candidate::Ignore
    }
}

#[must_use]
pub fn thread_name(author: &str, content: &str) -> String {
    let first_line = content.trim().lines().next().unwrap_or_default().trim();

    let name = if first_line.is_empty() {
        author.to_owned()
    } else {
        format!("{author} - {first_line}")
    };

    name.chars().take(THREAD_NAME_LIMIT).collect()
}

#[must_use]
pub fn parse_link(url: &str) -> Option<(ChannelId, MessageId)> {
    let mut segments = url.trim_end_matches('/').rsplit('/');

    let message = segments.next()?.parse::<u64>().ok().filter(|id| *id != 0)?;
    let channel = segments.next()?.parse::<u64>().ok().filter(|id| *id != 0)?;

    Some((ChannelId::new(channel), MessageId::new(message)))
}
