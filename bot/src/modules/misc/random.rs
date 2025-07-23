use async_trait::async_trait;
use rand::{rng, seq::IndexedRandom};
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, ResolvedOption,
    ResolvedValue,
};
use sqlx::{PgPool, Postgres};
use zayden_core::SlashCommand;

use crate::{Error, Result};

pub struct Random;

#[async_trait]
impl SlashCommand<Error, Postgres> for Random {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        let option = {
            let mut rng = rng();
            options.choose(&mut rng).unwrap()
        };

        let ResolvedValue::String(value) = option.value else {
            unreachable!("All options are strings")
        };

        let embed = CreateEmbed::new().description(format!("{}: {}", option.name, value));

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed),
                ),
            )
            .await
            .unwrap();

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        let cmd = CreateCommand::new("random")
            .description("A command demonstrating the maximum number of options (25).")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "1", "Option 1").required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "2", "Option 2").required(true),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "3",
                "Option 3",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "4",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "5",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "6",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "7",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "8",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "9",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "10",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "11",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "12",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "13",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "14",
                "Option ",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "15",
                "The fifteenth optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "16",
                "The sixteenth optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "17",
                "The seventeenth optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "18",
                "The eighteenth optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "19",
                "The nineteenth optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "20",
                "The twentieth optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "21",
                "The twenty-first optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "22",
                "The twenty-second optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "23",
                "The twenty-third optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "24",
                "The twenty-fourth optional string input.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "25",
                "The twenty-fifth optional string input.",
            ));

        Ok(cmd)
    }
}
