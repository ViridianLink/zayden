use futures::StreamExt;
use serenity::all::{GuildId, Http, Message, ThreadId};
use tracing::debug;

use crate::faq::facts::collapse;
use crate::faq::triage;
use crate::state::retitle;

const MESSAGE_LIMIT: usize = 200;
const MIN_MESSAGE_CHARS: usize = 15;
const TRANSCRIPT_LIMIT: usize = 12_000;

const USER_LABEL: &str = "User";
const BOT_LABEL: &str = "Support Bot";
const HELPER_LABEL: &str = "Helper";
const TITLE_LABEL: &str = "Thread title";
const DIAGNOSTIC_HEADER: &str = "Diagnostic questions asked:";
pub const OPENING_HEADER: &str = "Original issue:";
pub const CONVERSATION_HEADER: &str = "Conversation:";
const TRUNCATED: &str = "\n[truncated]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Human,
    TicketBody,
    Triage,
}

#[derive(Debug, Clone)]
pub struct RawMessage {
    pub author_id: u64,
    pub kind: MessageKind,
    pub content: String,
}

pub async fn collect(
    http: &Http,
    guild_id: GuildId,
    thread_id: ThreadId,
) -> Option<String> {
    let title = match thread_id.to_thread(http, Some(guild_id)).await {
        Ok(thread) => Some(retitle(&thread.base.name, "")),
        Err(e) => {
            debug!(error = ?e, %thread_id, "could not fetch thread title for faq");
            None
        },
    };

    let messages = thread_id
        .widen()
        .messages_iter(http)
        .take(MESSAGE_LIMIT)
        .filter_map(async |result| result.ok().as_ref().and_then(raw))
        .collect::<Vec<_>>()
        .await;

    render(title.as_deref(), &messages, TRANSCRIPT_LIMIT)
}

fn raw(message: &Message) -> Option<RawMessage> {
    if !message.author.bot() {
        return Some(RawMessage {
            author_id: message.author.id.get(),
            kind: MessageKind::Human,
            content: message.content.to_string(),
        });
    }

    triage_questions(message).or_else(|| ticket_body(message))
}

fn triage_questions(message: &Message) -> Option<RawMessage> {
    let embed = message
        .embeds
        .iter()
        .find(|embed| embed.title.as_deref() == Some(triage::EMBED_TITLE))?;

    let questions = embed
        .fields
        .iter()
        .find(|field| field.name.as_str() == triage::QUESTIONS_FIELD)?;

    Some(RawMessage {
        author_id: 0,
        kind: MessageKind::Triage,
        content: format!("{DIAGNOSTIC_HEADER}\n{}", questions.value),
    })
}

fn ticket_body(message: &Message) -> Option<RawMessage> {
    let body = message
        .embeds
        .iter()
        .filter_map(|embed| {
            let description = embed.description.as_deref()?;

            Some(embed.title.as_deref().map_or_else(
                || description.to_owned(),
                |title| format!("{title}: {description}"),
            ))
        })
        .collect::<Vec<_>>();

    if body.is_empty() {
        return None;
    }

    Some(RawMessage {
        author_id: message.mentions.first().map_or(0, |user| user.id.get()),
        kind: MessageKind::TicketBody,
        content: body.join("\n"),
    })
}

#[must_use]
pub fn render(
    title: Option<&str>,
    messages: &[RawMessage],
    limit: usize,
) -> Option<String> {
    let mut speakers = author(messages).map_or_default(|id| vec![id]);

    let lines = messages
        .iter()
        .rev()
        .filter(|message| keep(message))
        .map(|message| {
            let label = label(message, &mut speakers);
            (message.kind, format!("{label}: {}", message.content.trim()))
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return None;
    }

    let (opening, conversation) = split_opening(lines);

    let opening = title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(|title| format!("{TITLE_LABEL}: {title}"))
        .into_iter()
        .chain(opening)
        .collect::<Vec<_>>()
        .join("\n");

    let opening = format!(
        "{OPENING_HEADER}\n{}",
        zayden_core::text::truncate(&opening, limit / 2, TRUNCATED)
    );

    if conversation.is_empty() {
        return Some(opening);
    }

    let budget = limit
        .saturating_sub(opening.chars().count() + CONVERSATION_HEADER.len() + 3);

    Some(format!(
        "{opening}\n\n{CONVERSATION_HEADER}\n{}",
        tail(&conversation.join("\n"), budget)
    ))
}

fn split_opening(lines: Vec<(MessageKind, String)>) -> (Vec<String>, Vec<String>) {
    let has_body = lines.iter().any(|(kind, _)| *kind == MessageKind::TicketBody);
    let mut opening = Vec::new();
    let mut conversation = Vec::new();

    for (kind, line) in lines {
        let is_opening = if has_body {
            kind == MessageKind::TicketBody
        } else {
            opening.is_empty()
        };

        if is_opening {
            opening.push(line);
        } else {
            conversation.push(line);
        }
    }

    (opening, conversation)
}

#[must_use]
pub fn user_said(transcript: &str, quote: &str) -> bool {
    let quote = collapse(quote);

    if quote.is_empty() {
        return false;
    }

    let Some((_, conversation)) =
        transcript.split_once(&format!("\n\n{CONVERSATION_HEADER}\n"))
    else {
        return false;
    };

    let mut said = Vec::new();
    let mut current: Option<String> = None;

    for line in conversation.lines() {
        match speaker(line) {
            Some((label, content)) => {
                said.extend(current.take());
                current = (label == USER_LABEL).then(|| content.to_owned());
            },
            None => {
                if let Some(message) = current.as_mut() {
                    message.push('\n');
                    message.push_str(line);
                }
            },
        }
    }

    said.extend(current);
    said.iter().any(|message| collapse(message).contains(&quote))
}

fn speaker(line: &str) -> Option<(&str, &str)> {
    let (label, content) = line.split_once(": ")?;

    let known = label == USER_LABEL
        || label == BOT_LABEL
        || label
            .strip_prefix(HELPER_LABEL)
            .and_then(|index| index.strip_prefix(' '))
            .is_some_and(|index| index.parse::<usize>().is_ok());

    known.then_some((label, content))
}

fn author(messages: &[RawMessage]) -> Option<u64> {
    messages
        .iter()
        .rev()
        .find(|message| {
            message.kind == MessageKind::TicketBody && message.author_id != 0
        })
        .map(|message| message.author_id)
}

fn keep(message: &RawMessage) -> bool {
    message.kind != MessageKind::Human
        || message.content.trim().chars().count() >= MIN_MESSAGE_CHARS
}

fn label(message: &RawMessage, speakers: &mut Vec<u64>) -> String {
    match message.kind {
        MessageKind::Triage => return BOT_LABEL.to_owned(),
        MessageKind::TicketBody => return USER_LABEL.to_owned(),
        MessageKind::Human => {},
    }

    if !speakers.contains(&message.author_id) {
        speakers.push(message.author_id);
    }

    let index =
        speakers.iter().position(|id| *id == message.author_id).unwrap_or_default();

    if index == 0 {
        USER_LABEL.to_owned()
    } else {
        format!("{HELPER_LABEL} {index}")
    }
}

fn tail(transcript: &str, limit: usize) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut total = 0;

    for line in transcript.lines().rev() {
        let length = line.chars().count() + 1;

        if total + length > limit && !kept.is_empty() {
            break;
        }

        total += length;
        kept.push(line);
    }

    kept.reverse();
    kept.join("\n")
}
