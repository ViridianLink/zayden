use std::fmt::Write;

use jiff::{Span, Timestamp};
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
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::manager::GoldStarRow;
use crate::{GiveStar, GoldStarError, GoldStarManager, Result};

impl GiveStar {
    pub async fn run<Db: Database, Manager: GoldStarManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::User(target_user, _)) = options.remove("member")
        else {
            return Err(GoldStarError::InvalidOptions);
        };

        if interaction.user.id == target_user.id {
            return Err(GoldStarError::SelfStar);
        }

        let mut author_row = Manager::get_row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| GoldStarRow::new(interaction.user.id));
        let mut target_row = Manager::get_row(pool, target_user.id)
            .await?
            .unwrap_or_else(|| GoldStarRow::new(target_user.id));

        let next_free_star = author_row
            .last_free_star
            .to_jiff()
            .checked_add(Span::new().hours(24))
            .unwrap_or(Timestamp::MAX);

        let free_star = next_free_star <= Timestamp::now();

        if author_row.number_of_stars < 1 && !free_star {
            return Err(GoldStarError::NoStars(next_free_star.as_second()));
        }

        if free_star {
            author_row.give_free_star(&mut target_row);
        } else {
            author_row.give_star(&mut target_row);
        }

        author_row.save::<Db, Manager>(pool).await?;
        target_row.save::<Db, Manager>(pool).await?;

        let mut description = format!(
            "{} received a golden star from {} for a total of **{}** stars.",
            target_user.mention(),
            interaction.user.mention(),
            target_row.number_of_stars
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
