/*

                    if any(code in message.content.lower() for code in codewords):

                    else:
                        await
                else:
                    return
            else:
                return


*/

use serenity::all::{ChannelId, Context, Mentionable, Message, RoleId};

use crate::LLAMA_GUILD;

const CODEWORDS: [&str; 7] = [
    "password",
    "bonk",
    "fusion",
    "green man",
    "nova",
    "threadling",
    "buddy",
];
const CODEWORDS_CHANNEL: ChannelId = ChannelId::new(1388611694934097960);
const BEHIND_THE_SCENES_ROLE: RoleId = RoleId::new(1133034432148807680);
const BEHIND_THE_SCENES_CHANNEL: ChannelId = ChannelId::new(1133036087854497874);

pub struct BehindTheScenes;

impl BehindTheScenes {
    pub async fn run(ctx: &Context, message: &Message) {
        let Some(guild_id) = message.guild_id else {
            return;
        };

        if guild_id != LLAMA_GUILD || message.author.bot() {
            return;
        }

        if message.channel_id.expect_channel() != CODEWORDS_CHANNEL {
            return;
        }

        if !CODEWORDS
            .iter()
            .any(|code| message.content.eq_ignore_ascii_case(code))
        {
            message
                .reply(&ctx.http, "Incorrect codeword, please try again!")
                .await
                .unwrap();
        }

        ctx.http
            .add_member_role(
                guild_id,
                message.author.id,
                BEHIND_THE_SCENES_ROLE,
                Some("Correct codeword"),
            )
            .await
            .unwrap();

        message
            .reply(
                &ctx.http,
                format!(
                    "Access to {} has been granted!",
                    BEHIND_THE_SCENES_CHANNEL.mention()
                ),
            )
            .await
            .unwrap();
    }
}
