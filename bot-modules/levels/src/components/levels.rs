use serenity::all::{
    ComponentInteraction,
    Context,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use sqlx::PgPool;
use tracing::debug;

use crate::common::levels::{LeaderboardScope, create_embed};
use crate::{Levels, LevelsCustomId, LevelsError, RankRow, Result};

fn parse_footer(text: Option<&str>) -> Result<(i64, LeaderboardScope)> {
    let text = text.ok_or_else(|| {
        LevelsError::Internal("levels embed has no footer".to_string())
    })?;

    let rest = text.strip_prefix("Page ").ok_or_else(|| {
        LevelsError::Internal(
            "levels embed footer has unexpected format".to_string(),
        )
    })?;

    let (page_str, scope) = match rest.split_once(" · ") {
        Some((page, tag)) => (page, LeaderboardScope::from_footer_tag(tag)),
        None => (rest, LeaderboardScope::Guild),
    };

    let Ok(page) = page_str.trim().parse::<i64>() else {
        return Err(LevelsError::Internal(
            "levels embed footer page number not parseable".to_string(),
        ));
    };

    Ok((page, scope))
}

impl Levels {
    pub async fn run_components(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.unwrap_or_default();

        let Some(embed) = interaction.message.embeds.first() else {
            return Err(LevelsError::Internal(
                "levels message has no embed".to_string(),
            ));
        };

        let (current_page, scope) =
            parse_footer(embed.footer.as_ref().map(|f| f.text.as_str()))?;

        let page_number = match interaction
            .data
            .custom_id
            .parse::<LevelsCustomId>()?
        {
            LevelsCustomId::Previous => current_page - 1,
            LevelsCustomId::Next => current_page + 1,
            LevelsCustomId::User => {
                let rank = match scope {
                    LeaderboardScope::Guild => {
                        RankRow::guild_user_rank(pool, guild_id, interaction.user.id)
                            .await?
                    },
                    LeaderboardScope::Global => {
                        RankRow::user_rank(pool, interaction.user.id).await?
                    },
                };

                let Some(row_number) = rank else {
                    debug!("user has no rank entry");
                    return Ok(());
                };

                row_number / 10 + 1
            },
        }
        .max(1);

        let embed = create_embed(pool, guild_id, scope, page_number).await?;

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
