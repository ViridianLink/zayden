use ai::{
    chat::{Input, ResponseBody, Role},
    openai::OpenAI,
};
use serenity::all::{Context, GenericChannelId, Message};
use zayden_app::state::AppState;

use crate::{Error, RegistryBuilder, Result};

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

            let nested_contents = Self::process_referenced_messages(referenced_message);
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

            parsed_content = parsed_content.replace(&mention_tag, mention.display_name());
        }

        parsed_content
    }

    async fn reply(ctx: &Context, message: &Message, api_key: &str) -> Result<()> {
        let mut body = ResponseBody::basic().instructions(PERSONALITY);

        body = Self::process_referenced_messages(message).into_iter().fold(
            body,
            |body, (bot, content)| {
                body.input(Input::new(
                    content,
                    if bot { Role::Assistant } else { Role::User },
                ))
            },
        );

        body = body.input(Input::new(Self::parse_mentions(message), Role::User));

        let openai = OpenAI::new(api_key);
        let response = openai.create_response(&body).await?;

        let text = response
            .output
            .iter()
            .find_map(|output| {
                if output.kind == "message"
                    && let Some(content_vec) = &output.content
                    && let [content] = content_vec.as_slice()
                {
                    Some(&content.text)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                Error::Other("OpenAI response contained no usable message text".to_owned())
            })?;

        message.reply(&ctx.http, text).await?;
        Ok(())
    }

    pub async fn run(ctx: &Context, message: &Message, app: &AppState) -> Result<()> {
        const GAMBLING_CHANNEL: GenericChannelId = GenericChannelId::new(1281440730820116582);

        if message.channel_id != GAMBLING_CHANNEL {
            return Ok(());
        }

        if message
            .referenced_message
            .as_ref()
            .is_some_and(|msg| msg.content.is_empty())
        {
            return Ok(());
        }

        if !message.mentions_me(ctx).await.unwrap_or(false) {
            return Ok(());
        }

        if let Err(e) = Self::reply(ctx, message, &app.openai_api_key).await {
            tracing::error!(error = ?e, channel_id = %message.channel_id, "AI reply failed");
        }

        Ok(())
    }
}

pub fn register(_builder: &mut RegistryBuilder) {
    // ai has no slash commands — all interaction is via message events
}
