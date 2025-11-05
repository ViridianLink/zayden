use std::{
    fs::OpenOptions,
    io::{BufReader, Write},
};

use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, Context, CreateMessage, EmojiId, Message, ReactionType};

use crate::LLAMA_GUILD;

const COUNTING_CHANNEL: ChannelId = ChannelId::new(1386415868900020316);
const SADGE_EMOJI: EmojiId = EmojiId::new(1391921209884807299);
const FILE_NAME: &str = "countingFails.json";

pub struct CountingFail;

impl CountingFail {
    pub async fn run(ctx: &Context, message: &Message) {
        if message.guild_id.is_none_or(|guild| guild != LLAMA_GUILD) || message.author.bot() {
            return;
        }

        if message.channel_id.expect_channel() != COUNTING_CHANNEL
            || !message.content.contains(" RUINED IT AT ")
        {
            return;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .truncate(false)
            .write(true)
            .open(FILE_NAME)
            .unwrap();
        let data = BufReader::new(&file);

        let mut data =
            serde_json::from_slice::<CountingFailData>(data.buffer()).unwrap_or_default();
        data.counting_fails += 1;
        file.write_all(serde_json::to_string(&data).unwrap().as_bytes())
            .unwrap();

        message
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().content(format!(
                    "LlamaCord has ruined the count {} times {}",
                    data.counting_fails,
                    ReactionType::from(SADGE_EMOJI)
                )),
            )
            .await
            .unwrap();
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CountingFailData {
    counting_fails: u32,
}
