use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use serde::Deserialize;
use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter};
use zayden_app::state::AppState;

use crate::faq::embed::link_line;
use crate::faq::markdown::{self, DESCRIPTION_LIMIT};
use crate::wiki::{SearchResult, WikiConfig};

const SYSTEM_PROMPT: &str = "You are a Discord support-ticket triage assistant \
for a self-hosted documentation wiki. A new support ticket has just been opened \
with the user's message below, plus a fixed list of candidate wiki articles \
(title, description, path) found by a keyword search of that message.

Your job:
- Write a short, friendly greeting (1-2 sentences) acknowledging the user's issue.
- From the candidate articles ONLY, select the paths of the ones that genuinely help \
with this issue. Never invent an article or path that is not in the candidate list. \
It is fine to select none if nothing fits.
- Write 1 to 4 short follow-up triage questions a human helper would need answered \
before they can assist (e.g. software/version, exact error message, what was already \
tried). Skip a question the user's message already answers.

Do not use em dashes, emojis, or filler pleasantries. Be concise.";

const SCHEMA_NAME: &str = "triage_synthesis";
const MAX_TOKENS: u32 = 400;
const TEMPERATURE: f32 = 0.3;

const EMBED_TITLE: &str = "Automated Support Triage";
const EMBED_COLOUR: Colour = Colour::new(0x00_99_ff);
const EMBED_FOOTER: &str =
    "Please reply to this channel with the requested information.";
const ARTICLES_FIELD: &str = "Recommended Reading";
const QUESTIONS_FIELD: &str = "Action Required: Please Reply With";

/// Discord's per-field value budget.
const FIELD_LIMIT: usize = 1024;

#[derive(Deserialize)]
pub(crate) struct Triage {
    greeting: String,
    relevant_paths: Vec<String>,
    #[serde(rename = "triage_questions")]
    questions: Vec<String>,
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "greeting": { "type": "string" },
            "relevant_paths": { "type": "array", "items": { "type": "string" } },
            "triage_questions": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["greeting", "relevant_paths", "triage_questions"],
        "additionalProperties": false
    })
}

pub(crate) async fn synthesize(
    app: &AppState,
    message: &str,
    results: &[SearchResult],
) -> Result<Triage, ai::Error> {
    let candidates = results
        .iter()
        .map(|result| {
            serde_json::json!({
                "title": result.title,
                "description": result.description,
                "path": result.path,
            })
        })
        .collect::<Vec<_>>();

    let user_prompt = format!(
        "User's message: \"{message}\"\n\nCandidate wiki articles:\n{}",
        serde_json::Value::Array(candidates)
    );

    let messages = vec![
        ChatMessage::new(Role::System, SYSTEM_PROMPT),
        ChatMessage::new(Role::User, user_prompt),
    ];

    let client = AiClient::new(
        &app.ai_provider_key,
        &app.ai_api_endpoint,
        &app.ai_model_pro,
    )?;

    client
        .chat_json(messages, MAX_TOKENS, Some(TEMPERATURE), SCHEMA_NAME, schema())
        .await
}

pub(crate) fn embed(
    config: &WikiConfig,
    triage: &Triage,
    results: &[SearchResult],
) -> CreateEmbed<'static> {
    let mut embed = CreateEmbed::new()
        .title(EMBED_TITLE)
        .colour(EMBED_COLOUR)
        .description(markdown::truncate(&triage.greeting, DESCRIPTION_LIMIT))
        .footer(CreateEmbedFooter::new(EMBED_FOOTER));

    let articles = triage
        .relevant_paths
        .iter()
        .filter_map(|path| results.iter().find(|result| &result.path == path))
        .map(|result| link_line(config, result))
        .collect::<Vec<_>>();

    if !articles.is_empty() {
        embed = embed.field(
            ARTICLES_FIELD,
            markdown::truncate(&articles.join("\n"), FIELD_LIMIT),
            false,
        );
    }

    if !triage.questions.is_empty() {
        let questions = triage
            .questions
            .iter()
            .enumerate()
            .map(|(i, question)| format!("{}. {question}", i + 1))
            .collect::<Vec<_>>()
            .join("\n");

        embed = embed.field(
            QUESTIONS_FIELD,
            markdown::truncate(&questions, FIELD_LIMIT),
            false,
        );
    }

    embed
}
