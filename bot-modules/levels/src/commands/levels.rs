use serenity::all::{
    ButtonStyle,
    CommandInteraction,
    Context,
    CreateButton,
    CreateCommand,
    EditInteractionResponse,
};
use sqlx::PgPool;
use zayden_core::cache::GuildMembersCache;

use crate::common::levels::create_embed;
use crate::{Levels, LevelsCustomId, LevelsError, Result};

impl Levels {
    pub async fn run<Data: GuildMembersCache>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let Some(guild_id) = interaction.guild_id else {
            return Err(LevelsError::Internal(
                "command used outside a guild".to_string(),
            ));
        };

        let embed = create_embed::<Data>(ctx, pool, guild_id, 1).await?;

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed)
                    .button(
                        CreateButton::new(LevelsCustomId::Previous.as_str())
                            .label("<")
                            .style(ButtonStyle::Secondary),
                    )
                    .button(
                        CreateButton::new(LevelsCustomId::User.as_str())
                            .emoji('🎯')
                            .style(ButtonStyle::Secondary),
                    )
                    .button(
                        CreateButton::new(LevelsCustomId::Next.as_str())
                            .label(">")
                            .style(ButtonStyle::Secondary),
                    ),
            )
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("levels").description("Get the leaderboard")
    }
}
