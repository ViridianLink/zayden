use serenity::all::{ChannelId, Context, Mentionable, Message, RoleId};

use crate::{LlamaD2Error, Result};

const CODEWORDS: [&str; 7] =
    ["password", "bonk", "fusion", "green man", "nova", "threadling", "buddy"];
const CODEWORDS_CHANNEL: ChannelId = ChannelId::new(1_388_611_694_934_097_960);
const BEHIND_THE_SCENES_ROLE: RoleId = RoleId::new(1_133_034_432_148_807_680);
const BEHIND_THE_SCENES_CHANNEL: ChannelId =
    ChannelId::new(1_133_036_087_854_497_874);

pub struct BehindTheScenes;

impl BehindTheScenes {
    pub async fn run(ctx: &Context, message: &Message) -> Result<()> {
        let Some(guild_id) = message.guild_id else {
            return Err(LlamaD2Error::Internal(format!(
                "BehindTheScenes::run: message {} from user {} has no guild_id",
                message.id, message.author.id
            )));
        };

        if message.channel_id.expect_channel() != CODEWORDS_CHANNEL {
            return Err(LlamaD2Error::Internal(format!(
                "BehindTheScenes::run: message {} is not in the codewords channel",
                message.id
            )));
        }

        if !CODEWORDS.iter().any(|code| message.content.eq_ignore_ascii_case(code)) {
            return Err(LlamaD2Error::IncorrectCodeword);
        }

        ctx.http
            .add_member_role(
                guild_id,
                message.author.id,
                BEHIND_THE_SCENES_ROLE,
                Some("Correct codeword"),
            )
            .await?;

        message
            .reply(
                &ctx.http,
                format!(
                    "Access to {} has been granted!",
                    BEHIND_THE_SCENES_CHANNEL.mention()
                ),
            )
            .await?;

        Ok(())
    }
}
