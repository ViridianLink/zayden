// if any(code in message.content.lower() for code in codewords):
//
// else:
// await
// else:
// return
// else:
// return
//
//

use serenity::Error;
use serenity::all::{ChannelId, Context, Mentionable, Message, RoleId};

use crate::LLAMA_GUILD;

const CODEWORDS: [&str; 7] =
    ["password", "bonk", "fusion", "green man", "nova", "threadling", "buddy"];
const CODEWORDS_CHANNEL: ChannelId = ChannelId::new(1_388_611_694_934_097_960);
const BEHIND_THE_SCENES_ROLE: RoleId = RoleId::new(1_133_034_432_148_807_680);
const BEHIND_THE_SCENES_CHANNEL: ChannelId =
    ChannelId::new(1_133_036_087_854_497_874);

pub struct BehindTheScenes;

impl BehindTheScenes {
    pub async fn run(ctx: &Context, message: &Message) -> Result<(), Error> {
        let Some(guild_id) = message.guild_id else {
            return Ok(());
        };

        if guild_id != LLAMA_GUILD || message.author.bot() {
            return Ok(());
        }

        if message.channel_id.expect_channel() != CODEWORDS_CHANNEL {
            return Ok(());
        }

        if !CODEWORDS.iter().any(|code| message.content.eq_ignore_ascii_case(code)) {
            message
                .reply(&ctx.http, "Incorrect codeword, please try again!")
                .await?;

            return Ok(());
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
