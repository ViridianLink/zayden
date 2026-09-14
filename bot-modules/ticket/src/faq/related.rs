use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use serde::Deserialize;
use zayden_app::state::AppState;

use crate::faq::article::NewArticle;
use crate::faq::writer::{ARTICLE_FORMAT, WrittenArticle, article_schema};
use crate::faq::{facts, transcript};

const SYSTEM_PROMPT: &str = "You write the support FAQ for a Discord server. \
You are given the transcript of a support ticket \
that has just been marked solved, and the FAQ article already written about the \
issue the ticket was opened for. That article is finished. Your only job is to \
decide whether the conversation also fully solved a separate problem that \
deserves an article of its own.

The answer is almost always no. An empty entries list is the expected result, \
and returning nothing is always better than returning a weak article. Only \
write an entry for a problem when every one of these holds:
- It is a different problem from the one the existing article covers, not a \
step, cause, symptom or complication of it.
- The user actually ran into it during this ticket. It is not a \
hypothetical, a general question, or advice offered in passing.
- A fix was applied and the user said in the Conversation section that it \
worked. A helper saying it should work does not count.
- The transcript holds every step needed to repeat the fix. Nothing has to be \
guessed or filled in from your own knowledge.

Never write about a red herring: a suspected cause that turned out to be wrong, \
a change that did not help, or a detour that was abandoned. Never repeat \
anything the existing article already covers. Write at most two entries.

For each entry, set confirmation to the words the user wrote when they \
confirmed that fix worked, copied character for character from a User message \
in the Conversation section. Only use commands, paths, settings, versions and \
links that appear in the transcript, copied exactly.

Speakers are already anonymised. Never reintroduce a name, and never repeat an \
identifier, address, or credential that appears in the transcript.";

const SCHEMA_NAME: &str = "faq_related_articles";
const MAX_TOKENS: u32 = 3000;
const TEMPERATURE: f32 = 0.2;
pub const MAX_RELATED: usize = 2;
const MIN_CONFIRMATION_CHARS: usize = 8;

#[derive(Debug, Deserialize)]
pub struct RelatedEntry {
    #[serde(flatten)]
    pub article: WrittenArticle,
    pub confirmation: String,
}

#[derive(Debug, Deserialize)]
struct Related {
    entries: Vec<RelatedEntry>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Rejection {
    Unusable,
    OverLimit,
    Unconfirmed,
    Ungrounded(Vec<String>),
}

#[derive(Debug)]
pub struct Rejected {
    pub title: String,
    pub rejection: Rejection,
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "entries": {
                "type": "array",
                "items": article_schema(Some((
                    "confirmation",
                    serde_json::json!({ "type": "string" }),
                ))),
            }
        },
        "required": ["entries"],
        "additionalProperties": false
    })
}

#[must_use]
pub fn user_prompt(transcript: &str, primary: NewArticle<'_>) -> String {
    format!(
        "Existing article:\nTitle: {}\nSummary: {}\nBody:\n{}\n\nTicket \
         transcript:\n{transcript}",
        primary.title, primary.summary, primary.content
    )
}

pub(crate) async fn draft(
    app: &AppState,
    transcript: &str,
    primary: NewArticle<'_>,
) -> Result<Vec<RelatedEntry>, ai::Error> {
    let messages = vec![
        ChatMessage::new(
            Role::System,
            format!("{SYSTEM_PROMPT}\n\n{ARTICLE_FORMAT}"),
        ),
        ChatMessage::new(Role::User, user_prompt(transcript, primary)),
    ];

    let client = AiClient::new(
        &app.ai_provider_key,
        &app.ai_api_endpoint,
        &app.ai_model_pro,
    )?;

    let mut related: Related = client
        .chat_json(messages, MAX_TOKENS, Some(TEMPERATURE), SCHEMA_NAME, schema())
        .await?;

    for entry in &mut related.entries {
        entry.article.tidy();
    }

    Ok(related.entries)
}

pub fn vet(entry: &RelatedEntry, transcript: &str) -> Result<(), Rejection> {
    if !entry.article.is_usable() {
        return Err(Rejection::Unusable);
    }

    if entry.confirmation.trim().chars().count() < MIN_CONFIRMATION_CHARS
        || !transcript::user_said(transcript, &entry.confirmation)
    {
        return Err(Rejection::Unconfirmed);
    }

    let literals = facts::literals(&entry.article.markdown);
    let ungrounded = facts::missing(&literals, transcript);

    if !ungrounded.is_empty() {
        return Err(Rejection::Ungrounded(
            ungrounded.into_iter().map(str::to_owned).collect(),
        ));
    }

    Ok(())
}

#[must_use]
pub fn sift(
    entries: Vec<RelatedEntry>,
    transcript: &str,
) -> Vec<Result<WrittenArticle, Rejected>> {
    let mut accepted = 0;

    entries
        .into_iter()
        .map(|entry| {
            let verdict = if accepted == MAX_RELATED {
                Err(Rejection::OverLimit)
            } else {
                vet(&entry, transcript)
            };

            match verdict {
                Ok(()) => {
                    accepted += 1;
                    Ok(entry.article)
                },
                Err(rejection) => {
                    Err(Rejected { title: entry.article.title, rejection })
                },
            }
        })
        .collect()
}
