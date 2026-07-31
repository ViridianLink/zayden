use serenity::all::{
    ChannelId,
    Context,
    CreateMessage,
    EmojiId,
    Message,
    ReactionType,
};
use sqlx::PgPool;
use tracing::debug;

use crate::{Counter, Result};

const COUNTING_CHANNEL: ChannelId = ChannelId::new(1_386_415_868_900_020_316);
const SADGE_EMOJI: EmojiId = EmojiId::new(1_391_921_209_884_807_299);

pub struct CountingFail;

impl CountingFail {
    pub async fn run(ctx: &Context, message: &Message, pool: &PgPool) -> Result<()> {
        if message.channel_id.expect_channel() != COUNTING_CHANNEL
            || !message.content.contains(" RUINED IT AT ")
        {
            debug!(channel_id = %message.channel_id, "ignoring message: not a counting-fail report");
            return Ok(());
        }

        let counting_fails = Counter::bump(pool, Counter::COUNTING_FAILS).await?;

        message
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().content(format!(
                    "LlamaCord has ruined the count {} times {}",
                    counting_fails,
                    ReactionType::from(SADGE_EMOJI)
                )),
            )
            .await?;

        Ok(())
    }
}
