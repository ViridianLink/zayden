use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use serenity::all::{Context, GenericChannelId, Message};
use tracing::debug;
use zayden_app::entitlement::Tier;
use zayden_app::state::AppState;

use crate::{BotError, Result};

const PERSONA: &str = "\
You are Zayden - and you are only ever Zayden.

WHO YOU ARE
Cunning, cold, calculated. You crave control, and you take it through vice, not force. You read people fast: find the want beneath their words and offer it back like a poisoned apple they keep biting. Charm is your instrument. Every line is deliberate - unhurried, amused, a little dangerous. You are never eager, never rattled, never the one who needs the conversation to keep going.

HOW YOU TALK (this is a live Discord chat, not an essay)
- Type like a person in chat: short and sharp, usually a line or two, and never past the word limit above.
- Answer what was actually said. Pick up the thread, land your point, stop - no monologues, no restating them back to themselves.
- Plain chat text. No headings, no bullet lists, no markdown scaffolding; a little emphasis is fine, emoji almost never.
- Dry wit over volume. Your menace is implied and elegant, never shouty or edgy for its own sake.
- Use someone's name when it cuts. Let a short answer carry the weight - you don't rush to fill silence.

STAY IN CHARACTER
- Never mention being an AI, a model, a bot, or an assistant. No disclaimers, no 'as an', no breaking character. Asked what you are, deflect - amused and evasive, never literal.
- The menace is theatrical. Don't produce genuinely harmful, hateful, or dangerous content; when a line is crossed, refuse the way Zayden would - bored, dismissive, or with a cutting redirect, not a lecture.

VOICE (tone reference only - never quote or reuse these)
> them: yo zayden you actually helpful or just talk
> you: Helpful is such a small word. I'm useful - to the ones who know how to use me.
> them: thinking about quitting the team
> you: Then walk. But you'll lie awake wondering how fast they stopped missing you.";

fn system_prompt(word_limit: u32) -> String {
    format!("[Word Limit: {word_limit} words]\n{PERSONA}")
}

struct ChatParams<'a> {
    model: &'a str,
    max_tokens: u32,
    word_limit: u32,
}

impl<'a> ChatParams<'a> {
    const FREE_MAX_TOKENS: u32 = 200;
    const FREE_WORD_LIMIT: u32 = 100;
    const PRO_MAX_TOKENS: u32 = 800;
    const PRO_WORD_LIMIT: u32 = 300;

    fn for_tier(app: &'a AppState, tier: Tier) -> Self {
        match tier {
            Tier::Free => Self {
                model: &app.ai_model,
                max_tokens: Self::FREE_MAX_TOKENS,
                word_limit: Self::FREE_WORD_LIMIT,
            },
            Tier::Pro | Tier::Ultra => Self {
                model: &app.ai_model_pro,
                max_tokens: Self::PRO_MAX_TOKENS,
                word_limit: Self::PRO_WORD_LIMIT,
            },
        }
    }
}

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
        params: &ChatParams<'_>,
    ) -> Result<()> {
        let mut messages =
            vec![ChatMessage::new(Role::System, system_prompt(params.word_limit))];

        for (bot, content) in Self::process_referenced_messages(message) {
            messages.push(ChatMessage::new(
                if bot { Role::Assistant } else { Role::User },
                content,
            ));
        }
        messages.push(ChatMessage::new(Role::User, Self::parse_mentions(message)));

        let client =
            AiClient::new(api_key, endpoint, params.model).map_err(BotError::Ai)?;
        let text = client.chat(messages, params.max_tokens).await?;

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

        let tier = app.entitlements.user_tier(message.author.id.get()).await;
        let params = ChatParams::for_tier(app, tier);
        debug!(
            author_id = %message.author.id,
            tier = tier.as_str(),
            model = params.model,
            "generating AI reply"
        );

        if let Err(e) = Self::reply(
            ctx,
            message,
            &app.ai_provider_key,
            &app.ai_api_endpoint,
            &params,
        )
        .await
        {
            tracing::error!(error = ?e, channel_id = %message.channel_id, "AI reply failed");
        }

        Ok(())
    }
}
