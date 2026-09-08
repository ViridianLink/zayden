use std::fmt::Write as _;

use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use serde::Deserialize;
use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter};
use zayden_app::state::AppState;

use crate::faq::essential::Essential;
use crate::faq::hit::{FaqHit, FaqSource};
use crate::faq::linked::LinkedPage;
use crate::faq::render::truncate;
use crate::faq::view::{essential_line, link_line};
use crate::wiki::WikiConfig;

const SYSTEM_PROMPT: &str = "You are a Discord support-ticket triage assistant \
for a self-hosted documentation wiki. A new support ticket has just been opened. \
You are given its title, the forum tags the user picked, their message, the text \
of every page their message linked to, a fixed list of candidate wiki \
articles (title, description, path) found by a keyword search of that message, and \
some internal notes this server's helpers keep.

Your job:
- From the candidate wiki articles ONLY, select the paths of the ones that genuinely help \
with this issue. Never invent an article or path that is not in the candidate list, and \
never select an internal note - those are yours to read, not to hand to the user. \
It is fine to select none if nothing fits.
- Write 1 to 4 short follow-up triage questions a human helper would need answered \
before they can assist (e.g. software/version, exact error message, what was already \
tried). Return an empty list when nothing is left to ask.

Never ask for something the ticket already provides. The title, the forum tags and \
the linked page contents are part of the ticket: a tag naming the product, platform \
or version has answered that question, and a linked log, paste or issue has answered \
every question its text covers. Read the linked pages before deciding what to ask.

Do not use em dashes, emojis, or filler pleasantries. Be concise.";

const SCHEMA_NAME: &str = "triage_synthesis";
const MAX_TOKENS: u32 = 1200;
const TEMPERATURE: f32 = 0.3;

pub(crate) const EMBED_TITLE: &str = "Automated Triage";
const EMBED_DESCRIPTION: &str = "To help us provide the best possible support, please review the wiki pages linked below and answer the follow-up questions provided.";
const EMBED_COLOUR: Colour = Colour::new(0x00_99_ff);
const EMBED_FOOTER: &str =
    "Please reply to this channel with the requested information.";
const ARTICLES_FIELD: &str = "Recommended Reading";
pub(crate) const QUESTIONS_FIELD: &str = "Follow-up Questions";

const FIELD_LIMIT: usize = 1024;

#[derive(Deserialize)]
pub(crate) struct Triage {
    relevant_paths: Vec<String>,
    #[serde(rename = "triage_questions")]
    questions: Vec<String>,
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "relevant_paths": { "type": "array", "items": { "type": "string" } },
            "triage_questions": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["greeting", "relevant_paths", "triage_questions"],
        "additionalProperties": false
    })
}

#[derive(Debug, Clone, Copy)]
pub struct Opening<'a> {
    pub title: &'a str,
    pub tags: &'a [String],
    pub message: &'a str,
    pub links: &'a [LinkedPage],
}

#[must_use]
pub fn user_prompt(opening: Opening<'_>, hits: &[FaqHit]) -> String {
    let mut prompt = String::new();

    if !opening.title.trim().is_empty() {
        let _ = writeln!(prompt, "Ticket title: {}", opening.title.trim());
    }

    let _ = writeln!(
        prompt,
        "Forum tags the user applied: {}",
        if opening.tags.is_empty() {
            String::from("(none)")
        } else {
            opening.tags.join(", ")
        }
    );

    let _ = writeln!(prompt, "\nUser's message:\n{}", opening.message);

    for page in opening.links {
        let _ = writeln!(
            prompt,
            "\nContents of the page the user linked ({}):\n{}",
            page.url, page.text
        );
    }

    let (candidates, internal): (Vec<_>, Vec<_>) =
        hits.iter().partition(|hit| matches!(hit.source, FaqSource::Wiki));

    let _ = write!(
        prompt,
        "\nCandidate wiki articles:\n{}",
        serde_json::Value::Array(candidates.into_iter().map(describe).collect())
    );

    if !internal.is_empty() {
        let _ = write!(
            prompt,
            "\n\nInternal notes, for context only - never select these:\n{}",
            serde_json::Value::Array(internal.into_iter().map(describe).collect())
        );
    }

    prompt
}

fn describe(hit: &FaqHit) -> serde_json::Value {
    serde_json::json!({
        "title": hit.title,
        "description": hit.description,
        "path": hit.path,
    })
}

pub(crate) async fn synthesize(
    app: &AppState,
    opening: Opening<'_>,
    hits: &[FaqHit],
) -> Result<Triage, ai::Error> {
    let messages = vec![
        ChatMessage::new(Role::System, SYSTEM_PROMPT),
        ChatMessage::new(Role::User, user_prompt(opening, hits)),
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
    hits: &[FaqHit],
    essential: &[Essential],
) -> CreateEmbed<'static> {
    let mut embed = CreateEmbed::new()
        .title(EMBED_TITLE)
        .colour(EMBED_COLOUR)
        .description(EMBED_DESCRIPTION)
        .footer(CreateEmbedFooter::new(EMBED_FOOTER));

    let picked = triage
        .relevant_paths
        .iter()
        .filter_map(|path| hits.iter().find(|hit| &hit.path == path))
        .filter(|hit| matches!(hit.source, FaqSource::Wiki))
        .collect::<Vec<_>>();

    let articles = picked
        .iter()
        .map(|hit| link_line(config, hit))
        .chain(
            essential
                .iter()
                .filter(|page| !picked.iter().any(|hit| hit.path == page.path))
                .map(|page| essential_line(config, page)),
        )
        .collect::<Vec<_>>();

    if !articles.is_empty() {
        embed = embed.field(
            ARTICLES_FIELD,
            truncate(&articles.join("\n"), FIELD_LIMIT),
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

        embed =
            embed.field(QUESTIONS_FIELD, truncate(&questions, FIELD_LIMIT), false);
    }

    embed
}
