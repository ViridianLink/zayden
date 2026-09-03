use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use serde::Deserialize;
use zayden_app::state::AppState;

const SYSTEM_PROMPT: &str = "You are a technical writer for a self-hosted \
documentation wiki. You are given the transcript of a Discord support ticket \
that has just been marked solved. Turn it into one reusable FAQ article.

Speakers are already anonymised. Never reintroduce a name, and never repeat an \
identifier, address, or credential that appears in the transcript.

Structure the article body with these headers, in this order, omitting \
Prevention when you have nothing to say there:
## Problem
## Cause
## Solution
## Prevention

Write the Solution as numbered steps. Use code formatting for commands, paths, \
file names and settings. Keep it short: describe what was actually done, not \
what might have worked. Do not include greetings, thanks, or any other \
conversational filler. Do not use em dashes or emojis.

The title must name the specific symptom the way someone hitting it would \
search for it. The summary is one sentence, under 100 characters, and does not \
repeat the title verbatim.

If the transcript does not contain a solution anyone could follow, for example \
the user resolved it themselves without saying how, or the thread was closed \
without a fix, set status to insufficient_data and leave the other fields \
empty. Do not invent a solution.";

const SCHEMA_NAME: &str = "faq_article_draft";
const MAX_TOKENS: u32 = 1200;
const TEMPERATURE: f32 = 0.3;
const MAX_TAGS: usize = 6;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DraftStatus {
    Ok,
    InsufficientData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Draft {
    pub status: DraftStatus,
    pub title: String,
    pub summary: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub markdown: String,
}

impl Draft {
    pub(crate) fn tidy(&mut self) {
        self.title = self.title.trim().to_owned();
        self.summary = self.summary.trim().to_owned();
        self.markdown = self.markdown.trim().to_owned();
        self.category = self
            .category
            .as_deref()
            .map(str::trim)
            .filter(|category| !category.is_empty())
            .map(str::to_owned);

        let mut tags = Vec::with_capacity(MAX_TAGS);

        for tag in self.tags.drain(..) {
            let tag = tag.trim().to_lowercase();

            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }

        tags.truncate(MAX_TAGS);
        self.tags = tags;
    }

    pub(crate) fn is_usable(&self) -> bool {
        self.status == DraftStatus::Ok
            && !self.title.is_empty()
            && !self.markdown.is_empty()
    }
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "status": { "type": "string", "enum": ["ok", "insufficient_data"] },
            "title": { "type": "string" },
            "summary": { "type": "string" },
            "category": { "type": ["string", "null"] },
            "tags": { "type": "array", "items": { "type": "string" } },
            "markdown": { "type": "string" }
        },
        "required": [
            "status", "title", "summary", "category", "tags", "markdown"
        ],
        "additionalProperties": false
    })
}

pub(crate) async fn draft(
    app: &AppState,
    transcript: &str,
) -> Result<Draft, ai::Error> {
    let messages = vec![
        ChatMessage::new(Role::System, SYSTEM_PROMPT),
        ChatMessage::new(Role::User, format!("Ticket transcript:\n{transcript}")),
    ];

    let client = AiClient::new(
        &app.ai_provider_key,
        &app.ai_api_endpoint,
        &app.ai_model_pro,
    )?;

    let mut draft: Draft = client
        .chat_json(messages, MAX_TOKENS, Some(TEMPERATURE), SCHEMA_NAME, schema())
        .await?;

    draft.tidy();

    Ok(draft)
}
