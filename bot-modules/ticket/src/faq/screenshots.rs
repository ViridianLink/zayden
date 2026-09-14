use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use serenity::all::Attachment;
use zayden_app::state::AppState;

const SYSTEM_PROMPT: &str = "You read the images attached to a support ticket \
for a Discord server. They are usually screenshots of an error, a log, a console or \
a settings page.

For each image, in order:
- Transcribe verbatim every piece of text that could help diagnose the problem: \
error messages, log lines, stack traces, file paths, version numbers, settings and \
their values. Skip decorative or unrelated interface text.
- Then add one sentence saying what the image shows.

Label each image as \"Image 1:\", \"Image 2:\" and so on. Never guess at text you \
cannot read. If an image holds nothing relevant, say so in one line. Do not use em \
dashes, emojis, or filler pleasantries.";

const MAX_IMAGES: usize = 4;
const MAX_IMAGE_BYTES: u32 = 20 * 1024 * 1024;
const MAX_TOKENS: u32 = 2000;
const TEMPERATURE: f32 = 0.1;

const READABLE_TYPES: [&str; 3] = ["image/png", "image/jpeg", "image/webp"];
const READABLE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

#[must_use]
pub fn is_readable(content_type: Option<&str>, filename: &str, size: u32) -> bool {
    if size > MAX_IMAGE_BYTES {
        return false;
    }

    if let Some(content_type) = content_type {
        let kind = content_type.split(';').next().unwrap_or(content_type).trim();
        return READABLE_TYPES.iter().any(|t| kind.eq_ignore_ascii_case(t));
    }

    filename.rsplit_once('.').is_some_and(|(_, extension)| {
        READABLE_EXTENSIONS.iter().any(|e| extension.eq_ignore_ascii_case(e))
    })
}

#[must_use]
pub fn urls(attachments: &[Attachment]) -> Vec<String> {
    attachments
        .iter()
        .filter(|a| is_readable(a.content_type.as_deref(), &a.filename, a.size))
        .take(MAX_IMAGES)
        .map(|a| a.url.to_string())
        .collect()
}

pub(crate) async fn read(
    app: &AppState,
    title: &str,
    images: Vec<String>,
) -> Result<String, ai::Error> {
    let messages = vec![
        ChatMessage::new(Role::System, SYSTEM_PROMPT),
        ChatMessage::with_images(format!("Ticket title: {title}"), images),
    ];

    let client = AiClient::new(
        &app.ai_provider_key,
        &app.ai_api_endpoint,
        &app.ai_model_pro,
    )?;

    let text = client.chat(messages, MAX_TOKENS, Some(TEMPERATURE)).await?;

    Ok(text.trim().to_owned())
}
