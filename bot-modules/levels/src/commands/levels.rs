use serenity::all::{
    ButtonStyle,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateButton,
    CreateCommand,
    CreateCommandOption,
    EditInteractionResponse,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::PgPool;
use zayden_core::parse_options;

use crate::common::levels::{LeaderboardScope, create_embed};
use crate::{Levels, LevelsCustomId, Result};

impl Levels {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut options = parse_options(options);
        let global =
            matches!(options.remove("global"), Some(ResolvedValue::Boolean(true)));

        let scope = match interaction.guild_id {
            Some(_) => LeaderboardScope::from_global_flag(global),
            None => LeaderboardScope::Global,
        };
        let guild_id = interaction.guild_id.unwrap_or_default();

        let embed = create_embed(pool, guild_id, scope, 1).await?;

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
        CreateCommand::new("levels").description("Get the leaderboard").add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "global",
                "Show the cross-server global leaderboard instead of this server",
            ),
        )
    }
}
