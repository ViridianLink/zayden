use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, Permissions, ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{Error, Result};

const DESC: &str = "- LIFE COMES FIRST. We won't kick you or get upset for things that are happening in your life. We'll be here when you can play! With that said, if you are removed from the clan due to inactivity, you will be kept in the discord, and are welcome to ask one of the admins for a re-invite when you become active again; you're always welcome back!

- No Extreme Slurs or Targeted Discrimination, we do not allow the use of extreme slurs, hate speech, or language that attacks individuals or groups. This includes both direct harassment and indirect derogatory references.
  - Messages that violate this rule will be removed, and repeat or severe offenses may result in timeouts or permanent removal from the community.
  - Within VCs, we understand that especially within raids, words can be said. Just please know your audience, make sure that the people you are making these comments around will not take offence.

- Don't be afraid to ask for help when needed; we are here to support you.
We don't condone actions of hacks, cheats or any behaviour that breaches Bungie's Terms of Service, YOU WILL BE KICKED. This includes but is not limited to aim bots, distribution of 'cracked' clients, net limiting or tampered-with accounts. Discussing glitches and \"cheese strategies\" are exceptions to this rule.

- No spamming.  i.e. Excessive text chat and/or voice chat disruptions, including copypastas, playing loud music/soundboard effects (That means you Mo), etc.

- No using our Discord server or Clan, to promote other Clans, or promoting hate of another Clan/Guardian.

- Please be mindful to not spoil the game for others by using the appropriate Spoiler channel when necessary.

- We kindly ask that you attempt to schedule a raid with the clan before using public LFG, guests are more than welcome! If there isn't enough interest/availably for your run after you have asked the clan, you are more than welcome to and encouraged to ask public LFGs and even invite others in if they match the vibe. (This rule is mainly for raids/newer dungeons. If you'd rather use fireteam finder for old content like Dares, old campaigns, or recruiting etc, that's totally understandable).

- The clan is not an LFG pool. Please do not organize a raid through LFG and then ask the clan to fill the remaining 1-2 spots. 
[NOTE: Raids have the most success when they are scheduled through Zayden or Charlemagne .] 

- We do have teaching raids and will post frequently, if it is tagged as teaching it's encouraged to let newbies join first. If it is posted as KWTD please do not join if you need teaching.

- If there is a raid you would like to learn, please reach out to an Admin and we would be more than happy to schedule a teaching run for you!";

pub struct CustomMsg;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for CustomMsg {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        let embed = CreateEmbed::new()
            .title("Community Rules")
            .description(DESC);

        interaction
            .channel_id
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await
            .unwrap();

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Message sent")
                        .ephemeral(true),
                ),
            )
            .await
            .unwrap();

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        let cmd = CreateCommand::new("custom_msg")
            .description("Custom Messages")
            .default_member_permissions(Permissions::ADMINISTRATOR);

        Ok(cmd)
    }
}
