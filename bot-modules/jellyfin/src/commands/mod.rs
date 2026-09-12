pub mod about;
pub mod gaps;
pub mod guess;
pub mod letterboxd;
pub mod link;
pub mod request;
pub mod serializd;
pub mod unlink;

use std::sync::Arc;

use serenity::all::{CommandOptionType, CreateCommand, CreateCommandOption};
use zayden_core::{InvocationCtx, parse_options, parse_subcommand};

use crate::error::{JellyfinError, Result};
use crate::runtime::JellyfinRuntime;

const MAX_USERNAME_LENGTH: u16 = 64;

pub struct Jellyfin;

impl Jellyfin {
    pub fn register() -> CreateCommand<'static> {
        CreateCommand::new("jellyfin")
            .description("Your Jellyfin account and the server's library")
            .add_option(sub(
                "link",
                "Link your Jellyfin account with a one-time code (no password)",
            ))
            .add_option(sub(
                "unlink",
                "Forget the Jellyfin account linked to your Discord",
            ))
            .add_option(sub("about", "Show what is on the media server"))
            .add_option(
                sub("request", "Ask for something to be added to the server")
                    .add_sub_option(
                        string("title", "What to look for")
                            .required(true)
                            .set_autocomplete(true),
                    )
                    .add_sub_option(
                        string("seasons", "Which seasons, for a show")
                            .add_string_choice("All", "all")
                            .add_string_choice("Latest", "latest")
                            .add_string_choice("First", "first"),
                    ),
            )
            .add_option(
                sub("guess", "Guess the film from a poster or a redacted plot")
                    .add_sub_option(
                        string("mode", "How to play")
                            .required(true)
                            .add_string_choice("Visual", "visual")
                            .add_string_choice("Text", "text"),
                    )
                    .add_sub_option(
                        string("difficulty", "How hard to make it")
                            .add_string_choice("Easy", "easy")
                            .add_string_choice("Medium", "medium")
                            .add_string_choice("Hard", "hard"),
                    ),
            )
            .add_option(gaps_command())
            .add_option(
                sub(
                    "letterboxd",
                    "Hand Jellyscribe your Letterboxd login and sync now",
                )
                .add_sub_option(
                    string("username", "Your Letterboxd username").required(true),
                )
                .add_sub_option(
                    string("password", "Your Letterboxd password").required(true),
                ),
            )
            .add_option(
                sub(
                    "serializd",
                    "Hand Jellyscribe your Serializd login and sync now",
                )
                .add_sub_option(
                    string("email", "Your Serializd email (or username)")
                        .required(true),
                )
                .add_sub_option(
                    string("password", "Your Serializd password").required(true),
                ),
            )
    }

    pub async fn run(
        cx: &InvocationCtx<'_>,
        runtime: &Arc<JellyfinRuntime>,
    ) -> Result<()> {
        let (name, sub_options) = parse_subcommand(cx.interaction.data.options())?;
        let options = parse_options(sub_options);

        match name {
            "link" => link::run(cx, runtime).await,
            "unlink" => unlink::run(cx, runtime).await,
            "about" => about::run(cx, runtime).await,
            "request" => request::run(cx, runtime, options).await,
            "guess" => guess::run(cx, runtime, options).await,
            "gaps" => gaps::run(cx, runtime, options).await,
            "letterboxd" => letterboxd::run(cx, runtime, options).await,
            "serializd" => serializd::run(cx, runtime, options).await,
            _ => Err(JellyfinError::UnknownSubcommand(name.to_string())),
        }
    }
}

fn sub(
    name: &'static str,
    description: &'static str,
) -> CreateCommandOption<'static> {
    CreateCommandOption::new(CommandOptionType::SubCommand, name, description)
}

fn string(
    name: &'static str,
    description: &'static str,
) -> CreateCommandOption<'static> {
    CreateCommandOption::new(CommandOptionType::String, name, description)
}

fn gaps_command() -> CreateCommandOption<'static> {
    sub("gaps", "Films and shows you rated highly that the server is missing")
        .add_sub_option(
            string(
                "letterboxd",
                "Letterboxd username for films (I set Jellyscribe up with it too)",
            )
            .max_length(MAX_USERNAME_LENGTH),
        )
        .add_sub_option(
            string("serializd", "Serializd username for shows")
                .max_length(MAX_USERNAME_LENGTH),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Number,
                "min_rating",
                "Only suggest titles rated at least this many stars (default 3.5)",
            )
            .min_number_value(0.5)
            .max_number_value(5.0),
        )
}
