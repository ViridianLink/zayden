use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use serenity::all::{Context, GenericChannelId, Message};
use tracing::debug;
use zayden_app::state::AppState;

use crate::{BotError, RegistryBuilder, Result};

const PERSONALITY: &str = "[Word Limit: 100]
You're Zayden. Cunning, cold, and calculated, you waste no words; each one is a weapon. You don't crave war or chaos—you crave control, built not through force but through vice.

You calculate, you ensnare. You offer desire—a poisoned apple they keep biting, again and again.";

pub struct Ai;

impl Ai {
    fn process_referenced_messages(msg: &Message) -> Vec<(bool, String)> {
        let mut contents = Vec::new();

        if let Some(referenced_message) = &msg.referenced_message {
            contents.push((
                referenced_message.author.bot(),
                Self::parse_mentions(referenced_message),
            ));

            let nested_contents =
                Self::process_referenced_messages(referenced_message);
            contents.extend(nested_contents);
        }

        contents
    }

    fn parse_mentions(message: &Message) -> String {
        let mut parsed_content = message.content.to_string();

        for mention in &message.mentions {
            let mention_tag = format!("<@{}>", mention.id);

            if mention.bot() {
                parsed_content = parsed_content.replace(&mention_tag, "");
                continue;
            }

            parsed_content =
                parsed_content.replace(&mention_tag, mention.display_name());
        }

        parsed_content
    }

    async fn reply(
        ctx: &Context,
        message: &Message,
        api_key: &str,
        endpoint: &str,
        model: &str,
    ) -> Result<()> {
        let mut messages = vec![ChatMessage::new(Role::System, PERSONALITY)];

        for (bot, content) in Self::process_referenced_messages(message) {
            messages.push(ChatMessage::new(
                if bot { Role::Assistant } else { Role::User },
                content,
            ));
        }
        messages.push(ChatMessage::new(Role::User, Self::parse_mentions(message)));

        let client =
            AiClient::new(api_key, endpoint, model).map_err(BotError::Ai)?;
        let text = client.chat(messages, 200).await?;

        message.reply(&ctx.http, &text).await?;
        Ok(())
    }

    pub async fn run(
        ctx: &Context,
        message: &Message,
        app: &AppState,
    ) -> Result<()> {
        const GAMBLING_CHANNEL: GenericChannelId =
            GenericChannelId::new(1_281_440_730_820_116_582);

        if message.channel_id != GAMBLING_CHANNEL {
            debug!(channel_id = %message.channel_id, "message not in the AI channel; ignoring");
            return Ok(());
        }

        if message
            .referenced_message
            .as_ref()
            .is_some_and(|msg| msg.content.is_empty())
        {
            debug!(
                channel_id = %message.channel_id,
                "referenced message has no content; ignoring"
            );
            return Ok(());
        }

        if !message.mentions_me(ctx).await.unwrap_or(false) {
            debug!(
                channel_id = %message.channel_id,
                author_id = %message.author.id,
                "message does not mention the bot; ignoring"
            );
            return Ok(());
        }

        if let Err(e) = Self::reply(
            ctx,
            message,
            &app.ai_provider_key,
            &app.ai_api_endpoint,
            &app.ai_model,
        )
        .await
        {
            tracing::error!(error = ?e, channel_id = %message.channel_id, "AI reply failed");
        }

        Ok(())
    }
}

pub const fn register(_builder: &mut RegistryBuilder) {
    // ai has no slash commands — all interaction is via message events
}
