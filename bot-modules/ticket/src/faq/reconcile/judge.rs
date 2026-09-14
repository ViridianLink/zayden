use std::fmt::Write as _;

use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use serde::Deserialize;

use crate::faq::article::FaqArticle;
use crate::faq::reconcile::Subject;
use crate::faq::reconcile::prompt::{UNFAMILIAR_DETAILS, describe, system};

const ACTIONS: &str = "You maintain the support FAQ of a Discord server and \
keep it free of overlapping articles. You are given a new article and the \
existing articles a keyword search found similar to it. Choose exactly one \
action:

- create: the new article is about a different problem from every existing \
article. Sharing a product, a feature or a keyword is not the same problem. Set \
target_id to null.
- merge: the new article and one existing article are about the same problem, \
and the new article adds something the existing one lacks, such as another \
cause, another fix, extra steps, a command, a setting, a version, a caveat or \
newer information. Set target_id to that existing article.
- discard: the new article and one existing article are about the same \
problem, and the existing article already holds every fact in the new one. \
Only choose this when deleting the new article would lose nothing: not a step, \
command, path, setting, version, link or caveat. Set target_id to that existing \
article.";

const TIEBREAKS: &str = "If the articles disagree on anything, choose merge, \
never discard. When unsure between merge and discard, choose merge. When unsure \
whether two articles describe the same problem, choose create. Give the reason \
in one short sentence.";

const SCHEMA_NAME: &str = "faq_reconcile_verdict";
const MAX_TOKENS: u32 = 800;
const TEMPERATURE: f32 = 0.1;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Create,
    Merge,
    Discard,
}

#[derive(Debug, Deserialize)]
pub struct Verdict {
    pub action: Action,
    pub target_id: Option<i32>,
    pub reason: String,
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "action": { "type": "string", "enum": ["create", "merge", "discard"] },
            "target_id": { "type": ["integer", "null"] },
            "reason": { "type": "string" }
        },
        "required": ["action", "target_id", "reason"],
        "additionalProperties": false
    })
}

#[must_use]
pub fn user_prompt(
    subject: Subject<'_>,
    existing: &[FaqArticle],
    feedback: Option<&str>,
) -> String {
    let mut prompt = String::new();

    describe(&mut prompt, "New article", subject.article, subject.dated);

    for article in existing {
        let heading = format!("Existing article id {}", article.id);
        describe(&mut prompt, &heading, article.as_new(), article.dated());
    }

    if let Some(feedback) = feedback {
        let _ = write!(prompt, "Your previous answer was rejected: {feedback}");
    }

    prompt
}

#[must_use]
pub fn system_prompt(context: Option<&str>) -> String {
    system(&[ACTIONS, UNFAMILIAR_DETAILS, TIEBREAKS], context)
}

pub(crate) async fn decide(
    client: &AiClient,
    context: Option<&str>,
    subject: Subject<'_>,
    existing: &[FaqArticle],
    feedback: Option<&str>,
) -> Result<Verdict, ai::Error> {
    let messages = vec![
        ChatMessage::new(Role::System, system_prompt(context)),
        ChatMessage::new(Role::User, user_prompt(subject, existing, feedback)),
    ];

    client
        .chat_json(messages, MAX_TOKENS, Some(TEMPERATURE), SCHEMA_NAME, schema())
        .await
}
