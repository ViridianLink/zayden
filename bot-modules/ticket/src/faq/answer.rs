use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use zayden_app::state::AppState;

use crate::faq::markdown::{self, PROMPT_LIMIT};
use crate::faq::tuning::AnswerTuning;
use crate::wiki::Page;

const SYSTEM_PROMPT: &str = "You are a helpful Discord support bot.
Your task is to answer the user's question, using the provided wiki page content as your primary and authoritative source.
- Treat the wiki page content as ground truth. If Google Search grounding surfaces information that conflicts with it, the wiki wins.
- You may use Google Search grounding to fill gaps the wiki page doesn't cover, but check the wiki content first and prefer it whenever it answers the question.
- Be concise and direct (under 150 words if possible).
- Use Discord markdown sparingly: code lines and code blocks for commands/paths are good, but use bolding and other text styling only where absolutely necessary.
- Do not use em dashes, emojis, or other characters heavily used by AI models.
- If neither the wiki nor search results answer the question, say so directly: \"The wiki mentions this topic, but for full details, please read the article.\" Saying you don't know is better than guessing.
- Do not hallucinate or add information you can't support from the wiki or search results.";

pub(crate) async fn answer(
    app: &AppState,
    tuning: AnswerTuning,
    question: &str,
    page: &Page,
) -> Result<String, ai::Error> {
    let content = markdown::truncate(&page.content, PROMPT_LIMIT);

    let user_prompt =
        format!("User's Question: \"{question}\"\n\nWiki Page Content:\n{content}");

    let messages = vec![
        ChatMessage::new(Role::System, SYSTEM_PROMPT),
        ChatMessage::new(Role::User, user_prompt),
    ];

    let client = AiClient::new(
        &app.ai_provider_key,
        &app.ai_api_endpoint,
        &app.ai_model_pro,
    )?;

    client.chat(messages, tuning.max_tokens, Some(tuning.temperature)).await
}
