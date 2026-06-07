use serenity::all::{
    ComponentInteraction,
    Context,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use sqlx::{Database, Pool};
use zayden_core::GuildMembersCache;

use crate::common::levels::create_embed;
use crate::{Levels, LevelsManager, Result};

impl Levels {
    pub async fn run_components<
        Data: GuildMembersCache,
        Db: Database,
        Manager: LevelsManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let Some(embed) = interaction.message.embeds.first() else {
            return Ok(());
        };

        let page_number = match interaction.data.custom_id.as_str() {
            "levels_previous" => {
                let Some(footer) = embed.footer.as_ref() else {
                    return Ok(());
                };

                let Some(text) = footer.text.strip_prefix("Page ") else {
                    return Ok(());
                };

                let Ok(page) = text.parse::<i64>() else {
                    return Ok(());
                };

                page - 1
            },
            "levels_user" => {
                let Some(row_number) =
                    Manager::user_rank(pool, interaction.user.id).await?
                else {
                    return Ok(());
                };

                row_number / 10 + 1
            },
            "levels_next" => {
                let Some(footer) = embed.footer.as_ref() else {
                    return Ok(());
                };
                let Some(text) = footer.text.strip_prefix("Page ") else {
                    return Ok(());
                };
                let Ok(page) = text.parse::<i64>() else {
                    return Ok(());
                };
                page + 1
            },
            _ => return Ok(()),
        }
        .max(1);

        let Some(guild_id) = interaction.guild_id else {
            return Ok(());
        };

        let embed =
            create_embed::<Data, Db, Manager>(ctx, pool, guild_id, page_number)
                .await?;

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
