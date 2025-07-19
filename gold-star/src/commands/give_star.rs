use chrono::{Duration, Utc};
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption, CreateEmbed,
    EditInteractionResponse, Http, Mentionable, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::manager::GoldStarRow;
use crate::GiveStar;
use crate::{Error, GoldStarManager, Result};

impl GiveStar {
    pub async fn run<Db: Database, Manager: GoldStarManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let mut options = parse_options(options);

        let target_user = match options.remove("member") {
            Some(ResolvedValue::User(user, _)) => user,
            _ => unreachable!("User option is required"),
        };

        if interaction.user.id == target_user.id {
            return Err(Error::SelfStar);
        }

        let mut author_row = match Manager::get_row(pool, interaction.user.id).await.unwrap() {
            Some(stars) => stars,
            None => GoldStarRow::new(interaction.user.id),
        };
        let mut target_row = match Manager::get_row(pool, target_user.id).await.unwrap() {
            Some(stars) => stars,
            None => GoldStarRow::new(target_user.id),
        };

        let next_free_star = author_row.last_free_star + Duration::hours(24);

        let free_star = next_free_star <= Utc::now();

        if author_row.number_of_stars < 1 && !free_star {
            return Err(Error::NoStars(next_free_star.timestamp()));
        }

        if free_star {
            author_row.give_free_star(&mut target_row);
        } else {
            author_row.give_star(&mut target_row);
        }

        author_row.save::<Db, Manager>(pool).await.unwrap();
        target_row.save::<Db, Manager>(pool).await.unwrap();

        let mut description = format!(
            "{} received a golden star from {} for a total of **{}** stars.",
            target_user.mention(),
            interaction.user.mention(),
            target_row.number_of_stars
        );

        if let Some(ResolvedValue::String(reason)) = options.remove("reason") {
            description.push_str(&format!("\nReason: {reason}"));
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
            .await
            .unwrap();

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
