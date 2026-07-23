use std::fmt::Write;

use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    EditInteractionResponse,
    Http,
    Mentionable,
    ResolvedOption,
    ResolvedValue,
    User,
};
use sqlx::PgPool;
use zayden_core::{parse_options, required_option};

use crate::{GiveStar, GoldStarError, GoldStarRow, Result};

impl GiveStar {
    pub async fn run(
        http: &Http,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let mut options = parse_options(options);

        let (target_user, _): (&User, _) = required_option(&mut options, "member")?;

        if interaction.user.id == target_user.id {
            return Err(GoldStarError::SelfStar);
        }

        let target_stars =
            GoldStarRow::give_star(pool, interaction.user.id, target_user.id)
                .await?;

        let mut description = format!(
            "{} received a golden star from {} for a total of **{}** stars.",
            target_user.mention(),
            interaction.user.mention(),
            target_stars
        );

        if let Some(ResolvedValue::String(reason)) = options.remove("reason") {
            let _ = write!(description, "\nReason: {reason}");
        }

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().embed(
                    CreateEmbed::new()
                        .title("⭐ NEW GOLDEN STAR ⭐")
                        .description(description),
                ),
            )
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("give_star")
            .description("Give a user a star")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "member",
                    "The member to give a star to",
                )
                .required(true),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "reason",
                "The reason for giving a star",
            ))
    }
}
