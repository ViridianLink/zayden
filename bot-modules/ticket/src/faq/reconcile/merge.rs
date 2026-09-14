use std::fmt::Write as _;

use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;

use crate::faq::article::FaqArticle;
use crate::faq::reconcile::Subject;
use crate::faq::reconcile::prompt::{UNFAMILIAR_DETAILS, describe, system};
use crate::faq::writer::{ARTICLE_FORMAT, WrittenArticle, article_schema};

const INSTRUCTIONS: &str = "You maintain the support FAQ of a Discord server. \
Two FAQ articles describe the same problem. Merge them into one article that \
replaces both.

Keep every fact from both articles: every cause, step, command, path, setting, \
version, link and caveat. Do not drop a detail because it looks redundant, \
outdated or unfamiliar. Copy commands, paths, settings, versions and URLs \
character for character.

When the problem has more than one cause, give each cause its own fix. Where \
the articles disagree, keep both and use the ticket dates to say which is the \
current answer and which applied earlier.

Never introduce a name, identifier, address or credential.";

const SCHEMA_NAME: &str = "faq_merged_article";
const MAX_TOKENS: u32 = 3000;
const TEMPERATURE: f32 = 0.2;

#[must_use]
pub fn user_prompt(
    subject: Subject<'_>,
    target: &FaqArticle,
    missing: &[String],
) -> String {
    let mut prompt = String::new();

    describe(&mut prompt, "Existing article", target.as_new(), target.dated());
    describe(&mut prompt, "New article", subject.article, subject.dated);

    if !missing.is_empty() {
        let _ = write!(
            prompt,
            "Your previous merge dropped these details. Include each of them \
             exactly as written: {}",
            missing
                .iter()
                .map(|detail| format!("`{detail}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    prompt
}

#[must_use]
pub fn system_prompt(context: Option<&str>) -> String {
    system(&[INSTRUCTIONS, UNFAMILIAR_DETAILS, ARTICLE_FORMAT], context)
}

pub(crate) async fn write(
    client: &AiClient,
    context: Option<&str>,
    subject: Subject<'_>,
    target: &FaqArticle,
    missing: &[String],
) -> Result<WrittenArticle, ai::Error> {
    let messages = vec![
        ChatMessage::new(Role::System, system_prompt(context)),
        ChatMessage::new(Role::User, user_prompt(subject, target, missing)),
    ];

    let mut article: WrittenArticle = client
        .chat_json(
            messages,
            MAX_TOKENS,
            Some(TEMPERATURE),
            SCHEMA_NAME,
            article_schema(None),
        )
        .await?;

    article.tidy();

    Ok(article)
}
