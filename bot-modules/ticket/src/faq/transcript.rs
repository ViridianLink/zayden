use futures::StreamExt;
use serenity::all::{Http, Message, ThreadId};

use crate::faq::{embed, triage};

const MESSAGE_LIMIT: usize = 200;
const MIN_MESSAGE_CHARS: usize = 15;
const TRANSCRIPT_LIMIT: usize = 12_000;

const USER_LABEL: &str = "User";
const BOT_LABEL: &str = "Support Bot";
const DIAGNOSTIC_HEADER: &str = "Diagnostic questions asked:";

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

pub async fn collect(http: &Http, thread_id: ThreadId) -> Option<String> {
    let messages = thread_id
        .widen()
        .messages_iter(http)
        .take(MESSAGE_LIMIT)
        .filter_map(async |result| result.ok().as_ref().and_then(raw))
        .collect::<Vec<_>>()
        .await;

    render(&messages, TRANSCRIPT_LIMIT)
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
        .filter(|embed| embed.title.as_deref() != Some(embed::CREATED_TITLE))
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
pub fn render(messages: &[RawMessage], limit: usize) -> Option<String> {
    let mut speakers = author(messages).map(|id| vec![id]).unwrap_or_default();

    let lines = messages
        .iter()
        .rev()
        .filter(|message| keep(message))
        .map(|message| {
            let label = label(message, &mut speakers);
            format!("{label}: {}", message.content.trim())
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return None;
    }

    Some(tail(&lines.join("\n"), limit))
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

    if index == 0 { USER_LABEL.to_owned() } else { format!("Helper {index}") }
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
