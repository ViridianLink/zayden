use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use zayden_app::state::AppState;

use crate::faq::markdown::{self, PROMPT_LIMIT};
use crate::faq::tuning::AnswerTuning;

const SYSTEM_PROMPT: &str = "You are a helpful Discord support bot.
Your task is to answer the user's question, using the provided reference article as your primary and authoritative source.
- Treat the reference article as ground truth. If Google Search grounding surfaces information that conflicts with it, the article wins.
- You may use Google Search grounding to fill gaps the article doesn't cover, but check the article first and prefer it whenever it answers the question.
- Be concise and direct (under 150 words if possible).
- Use Discord markdown sparingly: code lines and code blocks for commands/paths are good, but use bolding and other text styling only where absolutely necessary.
- Do not use em dashes, emojis, or other characters heavily used by AI models.
- If neither the article nor search results answer the question, say so directly: \"The documentation mentions this topic, but for full details, please read the article.\" Saying you don't know is better than guessing.
- Do not hallucinate or add information you can't support from the article or search results.";

pub(crate) async fn answer(
    app: &AppState,
    tuning: AnswerTuning,
    question: &str,
    content: &str,
) -> Result<String, ai::Error> {
    let content = markdown::truncate(content, PROMPT_LIMIT);

    let user_prompt =
        format!("User's Question: \"{question}\"\n\nReference Article:\n{content}");

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
