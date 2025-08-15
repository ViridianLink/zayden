use serenity::all::{Context, CreateMessage, GenericChannelId, Message, UserId};
use serenity::small_fixed_array::FixedString;
use tokio::sync::RwLock;

use crate::LLAMA_GUILD;

const GOOD_MORNINGS: [&str; 8] = [
    "good morning",
    "gm",
    "goodmorning",
    "good mornin",
    "mornin",
    "morning",
    "g'mornin",
    "g morn",
];

pub trait GoodMorningCache: Send + Sync + 'static {
    fn insert(
        &mut self,
        channel_id: GenericChannelId,
        author: UserId,
        content: FixedString<u16>,
    ) -> Option<(UserId, FixedString<u16>)>;
}

pub struct GoodMorning;

impl GoodMorning {
    pub async fn run<Data: GoodMorningCache>(ctx: &Context, message: &Message) {
        if message.guild_id.is_none_or(|guild| guild != LLAMA_GUILD) || message.author.bot() {
            return;
        }

        let prev_msg = {
            let data = ctx.data::<RwLock<Data>>();
            let mut data = data.write().await;
            data.insert(
                message.channel_id,
                message.author.id,
                message.content.clone(),
            )
        };

        if is_good_morning(&message.content)
            && prev_msg.is_some_and(|(last_author, content)| {
                last_author != message.author.id && is_good_morning(&content)
            })
        {
            message
                .channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new()
                        .content("Good Morning dear viewer <:GROG:1393906582298955776>"),
                )
                .await
                .unwrap();
        }
    }
}

fn is_good_morning(content: &str) -> bool {
    let trimmed_content = content.trim();

    GOOD_MORNINGS
        .iter()
        .any(|gm_prefix| trimmed_content.starts_with(gm_prefix))
}
