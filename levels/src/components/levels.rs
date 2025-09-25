use serenity::all::{
    ComponentInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use sqlx::{Database, Pool};
use zayden_core::GuildMembersCache;

use crate::{common::levels::create_embed, Levels, LevelsManager};

impl Levels {
    pub async fn run_components<
        Data: GuildMembersCache,
        Db: Database,
        Manager: LevelsManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) {
        let Some(embed) = interaction.message.embeds.first() else {
            unreachable!("Embed must be present")
        };

        let page_number = match interaction.data.custom_id.as_str() {
            "levels_previous" => {
                embed
                    .footer
                    .as_ref()
                    .unwrap()
                    .text
                    .strip_prefix("Page ")
                    .unwrap()
                    .parse::<i64>()
                    .unwrap()
                    - 1
            }
            "levels_user" => {
                let Some(row_number) = Manager::user_rank(pool, interaction.user.id).await.unwrap()
                else {
                    return;
                };

                row_number / 10 + 1
            }
            "levels_next" => {
                embed
                    .footer
                    .as_ref()
                    .unwrap()
                    .text
                    .strip_prefix("Page ")
                    .unwrap()
                    .parse::<i64>()
                    .unwrap()
                    + 1
            }
            _ => unreachable!(),
        }
        .max(1);

        let embed = create_embed::<Data, Db, Manager>(
            ctx,
            pool,
            interaction.guild_id.unwrap(),
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
            .await
            .unwrap();
    }
}
