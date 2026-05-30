use serenity::all::{
    ComponentInteraction,
    Context,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use sqlx::{Database, Pool};
use zayden_core::GuildMembersCache;

use crate::common::levels::create_embed;
use crate::{Levels, LevelsManager};

impl Levels {
    pub async fn run_components<
        Data: GuildMembersCache,
        Db: Database,
        Manager: LevelsManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> serenity::Result<()> {
        let Some(embed) = interaction.message.embeds.first() else {
            return Ok(());
        };

        let page_number = match interaction.data.custom_id.as_str() {
            "levels_previous" => {
                embed
                    .footer
                    .as_ref()
                    .expect("embed always has a footer with page number")
                    .text
                    .strip_prefix("Page ")
                    .expect("footer text always starts with 'Page '")
                    .parse::<i64>()
                    .expect("page number is always a valid i64")
                    - 1
            },
            "levels_user" => {
                let Some(row_number) = Manager::user_rank(pool, interaction.user.id)
                    .await
                    .expect("DB query")
                else {
                    return Ok(());
                };

                row_number / 10 + 1
            },
            "levels_next" => {
                embed
                    .footer
                    .as_ref()
                    .expect("embed always has a footer with page number")
                    .text
                    .strip_prefix("Page ")
                    .expect("footer text always starts with 'Page '")
                    .parse::<i64>()
                    .expect("page number is always a valid i64")
                    + 1
            },
            _ => return Ok(()),
        }
        .max(1);

        let embed = create_embed::<Data, Db, Manager>(
            ctx,
            pool,
            interaction.guild_id.expect("levels component always used in guild"),
            page_number,
        )
        .await;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().embed(embed),
                ),
            )
            .await?;

        Ok(())
    }
}
