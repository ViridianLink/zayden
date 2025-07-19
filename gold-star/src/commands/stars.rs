use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption, CreateEmbed,
    EditInteractionResponse, Http, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::{GoldStarManager, GoldStarRow, Result, Stars};

impl Stars {
    pub async fn run<Db: Database, Manager: GoldStarManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let mut options = parse_options(options);

        let user = match options.remove("user") {
            Some(ResolvedValue::User(user, _)) => user,
            _ => &interaction.user,
        };

        let row = match Manager::get_row(pool, user.id).await.unwrap() {
            Some(row) => row,
            None => GoldStarRow::new(user.id),
        };

        let username = user.global_name.as_deref().unwrap_or(&user.name);

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().embed(
                    CreateEmbed::new()
                        .title(format!("{username}'s Stars"))
                        .field("Number of Stars", row.number_of_stars.to_string(), true)
                        .field("Given Stars", row.given_stars.to_string(), true)
                        .field("Received Stars", row.received_stars.to_string(), true),
                ),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("stars")
            .description("Get the number of stars a user has.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "The user to get the stars for.",
                )
                .required(false),
            )
    }
}
