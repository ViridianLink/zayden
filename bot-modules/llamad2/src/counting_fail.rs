use std::fs::OpenOptions;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use serenity::Error;
use serenity::all::{
    ChannelId,
    Context,
    CreateMessage,
    EmojiId,
    Message,
    ReactionType,
};
use tracing::error;

use crate::LLAMA_GUILD;

const COUNTING_CHANNEL: ChannelId = ChannelId::new(1_386_415_868_900_020_316);
const SADGE_EMOJI: EmojiId = EmojiId::new(1_391_921_209_884_807_299);
const FILE_NAME: &str = "countingFails.json";

pub struct CountingFail;

impl CountingFail {
    pub async fn run(ctx: &Context, message: &Message) -> Result<(), Error> {
        if message.guild_id.is_none_or(|guild| guild != LLAMA_GUILD)
            || message.author.bot()
        {
            return Ok(());
        }

        if message.channel_id.expect_channel() != COUNTING_CHANNEL
            || !message.content.contains(" RUINED IT AT ")
        {
            return Ok(());
        }

        let Ok(mut file) = OpenOptions::new()
            .create(true)
            .read(true)
            .truncate(false)
            .write(true)
            .open(FILE_NAME)
        else {
            error!("Failed to open {FILE_NAME}");
            return Ok(());
        };

        let mut buffer = String::new();
        if file.read_to_string(&mut buffer).is_err() {
            error!("Failed to read {FILE_NAME}");
            return Ok(());
        }

        let mut data = match serde_json::from_str::<CountingFailData>(&buffer) {
            Ok(data) => data,
            Err(e) => {
                error!("Serde error: {e}");
                CountingFailData::default()
            },
        };

        data.counting_fails += 1;

        let Ok(serialized) = serde_json::to_string(&data) else {
            error!("Failed to serialize CountingFailData");
            return Ok(());
        };

        if file.set_len(0).is_err() || file.write_all(serialized.as_bytes()).is_err()
        {
            error!("Failed to write {FILE_NAME}");
            return Ok(());
        }

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
            .await?;

        Ok(())
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CountingFailData {
    counting_fails: u32,
}
