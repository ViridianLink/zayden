use serenity::all::{
    ComponentInteraction,
    Context,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use sqlx::PgPool;
use tracing::debug;
use zayden_core::GuildMembersCache;

use crate::common::levels::create_embed;
use crate::{Levels, LevelsCustomId, LevelsError, RankRow, Result};

impl Levels {
    pub async fn run_components<Data: GuildMembersCache>(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let Some(guild_id) = interaction.guild_id else {
            return Err(LevelsError::Internal(
                "component used outside a guild".to_string(),
            ));
        };

        let Some(embed) = interaction.message.embeds.first() else {
            return Err(LevelsError::Internal(
                "levels message has no embed".to_string(),
            ));
        };

        let page_number =
            match interaction.data.custom_id.parse::<LevelsCustomId>()? {
                LevelsCustomId::Previous => {
                    let Some(footer) = embed.footer.as_ref() else {
                        return Err(LevelsError::Internal(
                            "levels embed has no footer".to_string(),
                        ));
                    };

                    let Some(text) = footer.text.strip_prefix("Page ") else {
                        return Err(LevelsError::Internal(
                            "levels embed footer has unexpected format".to_string(),
                        ));
                    };

                    let Ok(page) = text.parse::<i64>() else {
                        return Err(LevelsError::Internal(
                            "levels embed footer page number not parseable"
                                .to_string(),
                        ));
                    };

                    page - 1
                },
                LevelsCustomId::User => {
                    let Some(row_number) =
                        RankRow::user_rank(pool, interaction.user.id).await?
                    else {
                        debug!("user has no rank entry");
                        return Ok(());
                    };

                    row_number / 10 + 1
                },
                LevelsCustomId::Next => {
                    let Some(footer) = embed.footer.as_ref() else {
                        return Err(LevelsError::Internal(
                            "levels embed has no footer".to_string(),
                        ));
                    };

                    let Some(text) = footer.text.strip_prefix("Page ") else {
                        return Err(LevelsError::Internal(
                            "levels embed footer has unexpected format".to_string(),
                        ));
                    };

                    let Ok(page) = text.parse::<i64>() else {
                        return Err(LevelsError::Internal(
                            "levels embed footer page number not parseable"
                                .to_string(),
                        ));
                    };

                    page + 1
                },
            }
            .max(1);

        let embed = create_embed::<Data>(ctx, pool, guild_id, page_number).await?;

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
