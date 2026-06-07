use std::borrow::Cow;

use async_trait::async_trait;
use rand::rng;
use rand::seq::IndexedRandom;
use serenity::all::{
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    ResolvedValue,
};
use zayden_core::{HandlerError, InvocationCtx, ModuleCommand};

pub(super) struct Random;

#[async_trait]
impl ModuleCommand for Random {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("random")
    }

    fn definition(&self) -> CreateCommand<'static> {
        CreateCommand::new("random")
            .description(
                "A command demonstrating the maximum number of options (25).",
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "1", "Option 1")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "2", "Option 2")
                    .required(true),
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
            ))
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();

        let pick = {
            let mut rng = rng();
            options.choose(&mut rng).and_then(|option| {
                if let ResolvedValue::String(v) = option.value {
                    Some((option.name.to_string(), v.to_string()))
                } else {
                    None
                }
            })
        };
        let Some((option_name, value)) = pick else {
            return Ok(());
        };

        let embed =
            CreateEmbed::new().description(format!("{option_name}: {value}"));

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed),
                ),
            )
            .await?;

        Ok(())
    }
}
