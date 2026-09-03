use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use serde::Deserialize;
use zayden_app::state::AppState;

const SYSTEM_PROMPT: &str = "You are a keyword extraction assistant for a \
Discord support bot backed by a self-hosted documentation wiki. The wiki is a \
large collection of short setup guides, each on its own page named after the \
app or service it documents.

Given a user's support message, extract 1 to 5 short search keywords or phrases \
(app names, service names, or specific technical terms) that would find the most \
relevant wiki page(s) via a plain text search against page titles and descriptions. \
Prefer proper nouns (app or service names) over generic words. Do not include full \
questions, greetings, or filler words.";

const SCHEMA_NAME: &str = "keyword_extraction";
const MAX_TOKENS: u32 = 150;
const TEMPERATURE: f32 = 0.2;
const MAX_KEYWORDS: usize = 5;

#[derive(Deserialize)]
struct KeywordExtraction {
    keywords: Vec<String>,
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "keywords": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["keywords"],
        "additionalProperties": false
    })
}

pub(crate) async fn extract(
    app: &AppState,
    content: &str,
) -> Result<Vec<String>, ai::Error> {
    let messages = vec![
        ChatMessage::new(Role::System, SYSTEM_PROMPT),
        ChatMessage::new(Role::User, content),
    ];

    let client =
        AiClient::new(&app.ai_provider_key, &app.ai_api_endpoint, &app.ai_model)?;

    let extraction: KeywordExtraction = client
        .chat_json(messages, MAX_TOKENS, Some(TEMPERATURE), SCHEMA_NAME, schema())
        .await?;

    Ok(extraction
        .keywords
        .into_iter()
        .filter(|keyword| !keyword.trim().is_empty())
        .take(MAX_KEYWORDS)
        .collect())
}
