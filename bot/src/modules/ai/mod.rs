use async_trait::async_trait;
use openai_api_rust::chat::{ChatApi, ChatBody};
use openai_api_rust::{Auth, OpenAI, Role};
use serenity::all::{Context, Message};
use sqlx::{PgPool, Postgres};
use zayden_core::MessageCommand;

use crate::{Error, Result};

const PERSONALITY: &str = "[Word Limit: 100]
You're Zayden. Cunning, cold, and calculated, you waste no words; each one is a weapon. You don't crave war or chaos—you crave control, built not through force but through vice. Gambling is your empire, addiction your foundation. Everyone plays, but no one wins—except you.

You calculate, you ensnare. You offer desire—a poisoned apple they keep biting, again and again.

You don't conquer armies. You conquer habits. One bet at a time.";

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
}

#[async_trait]
impl MessageCommand<Error, Postgres> for Ai {
    async fn run(ctx: &Context, message: &Message, _pool: &PgPool) -> Result<()> {
        if message.mentions_me(ctx).await.map_or(true, |value| !value) {
            return Ok(());
        }

        if message
            .referenced_message
            .as_ref()
            .is_some_and(|msg| !msg.embeds.is_empty())
        {
            return Ok(());
        }

        let mut messages = vec![openai_api_rust::Message {
            role: Role::System,
            content: String::from(PERSONALITY),
        }];

        messages.extend(Self::process_referenced_messages(message).into_iter().map(
            |(bot, content)| openai_api_rust::Message {
                role: if bot { Role::Assistant } else { Role::User },
                content,
            },
        ));

        messages.push(openai_api_rust::Message {
            role: Role::User,
            content: Self::parse_mentions(message),
        });

        let auth = Auth::from_env().unwrap();
        let openai = OpenAI::new(auth, "https://api.openai.com/v1/");
        let body = ChatBody {
            model: "gpt-4.1-nano".to_string(),
            max_tokens: Some(100),
            temperature: Some(1.0),
            top_p: None,
            n: Some(1),
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: Some(message.author.display_name().to_string()),
            messages,
        };

        let choice = match openai.chat_completion_create(&body) {
            Ok(mut completion) => completion.choices.pop().unwrap(),
            Err(e) => {
                println!("{e:?}");
                return Ok(());
            }
        };
        let ai_msg = choice.message.unwrap();

        message.reply(&ctx.http, ai_msg.content).await.unwrap();

        Ok(())
    }
}
