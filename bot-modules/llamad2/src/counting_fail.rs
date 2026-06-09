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
use tracing::debug;

const COUNTING_CHANNEL: ChannelId = ChannelId::new(1_386_415_868_900_020_316);
const SADGE_EMOJI: EmojiId = EmojiId::new(1_391_921_209_884_807_299);
const FILE_NAME: &str = "countingFails.json";

pub struct CountingFail;

impl CountingFail {
    pub async fn run(ctx: &Context, message: &Message) -> Result<(), Error> {
        if message.channel_id.expect_channel() != COUNTING_CHANNEL
            || !message.content.contains(" RUINED IT AT ")
        {
            debug!(channel_id = %message.channel_id, "ignoring message: not a counting-fail report");
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .truncate(false)
            .write(true)
            .open(FILE_NAME)?;

        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;

        let mut data: CountingFailData = serde_json::from_str(&buffer)?;

        data.counting_fails += 1;

        let serialized = serde_json::to_string(&data)?;

        file.set_len(0)?;
        file.write_all(serialized.as_bytes())?;

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
