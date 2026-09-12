pub mod binge;
pub mod justwatch;
pub mod party;
pub mod privacy;
pub mod recommend;
pub mod streak;
pub mod trivia;
pub mod warnings;
use std::collections::HashMap;
use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    ResolvedValue,
};
use zayden_core::{InvocationCtx, parse_options, parse_subcommand};

use crate::error::{Result, WatchError};
use crate::party::Scheduled;
pub struct Watch;

impl Watch {
    pub fn register() -> CreateCommand<'static> {
        CreateCommand::new("watch")
            .description("Plan what to watch and see what you have watched")
            .add_option(
                sub("justwatch", "See where something streams in your region")
                    .add_sub_option(
                        string("title", "What to look for")
                            .required(true)
                            .set_autocomplete(true),
                    ),
            )
            .add_option(
                sub("binge", "Work out how long a series takes")
                    .add_sub_option(
                        string("show", "Which series")
                            .required(true)
                            .set_autocomplete(true),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::Integer,
                            "per_day",
                            "Episodes per day (default 2)",
                        )
                        .min_int_value(1)
                        .max_int_value(10),
                    )
                    .add_sub_option(CreateCommandOption::new(
                        CommandOptionType::Boolean,
                        "skip_intros",
                        "Subtract measured intros and credits (default true)",
                    )),
            )
            .add_option(
                sub("warnings", "Community content warnings for a title")
                    .add_sub_option(
                        string("title", "What to look for")
                            .required(true)
                            .set_autocomplete(true),
                    ),
            )
            .add_option(
                sub("streak", "Show consecutive days watched").add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "Whose streak to show (defaults to you)",
                    ),
                ),
            )
            .add_option(
                sub("privacy", "Choose whether others can see your watch streak")
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::Boolean,
                            "visible",
                            "True to let others run /watch streak on you",
                        )
                        .required(true),
                    ),
            )
            .add_option(
                sub("trivia", "A question from watch history")
                    .add_sub_option(
                        string("scope", "Whose history to draw on")
                            .required(true)
                            .add_string_choice("Mine", "me")
                            .add_string_choice("The server's", "server"),
                    )
                    .add_sub_option(
                        string("tier", "How far back to look")
                            .add_string_choice("Last watched", "deep-dive")
                            .add_string_choice("Recent", "recent")
                            .add_string_choice("Everything", "yearly"),
                    ),
            )
            .add_option(recommend_group())
            .add_option(party_group())
    }

    pub async fn run(
        cx: &InvocationCtx<'_>,
        runtime: &Arc<JellyfinRuntime>,
    ) -> Result<Scheduled> {
        let (name, sub_options) = parse_subcommand(cx.interaction.data.options())?;

        match name {
            "recommend" => {
                recommend::run(cx, runtime).await?;
                return Ok(Scheduled::Nothing);
            },
            "party" => return party::run(cx, runtime).await,
            _ => {},
        }

        let options = parse_options(sub_options);

        // Split in two rather than one wide match: the combined future of every
        // handler inlined into a single `async fn` is large enough that the
        // compiler stops being able to prove its higher-ranked lifetime bound at
        // the `async_trait` call site (rust-lang/rust#100013).
        if is_library_subcommand(name) {
            library(cx, runtime, name, options).await?;
        } else {
            history(cx, runtime, name, options).await?;
        }

        Ok(Scheduled::Nothing)
    }
}

fn is_library_subcommand(name: &str) -> bool {
    matches!(name, "justwatch" | "binge" | "warnings")
}

async fn library(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    name: &str,
    options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    match name {
        "justwatch" => justwatch::run(cx, runtime, options).await,
        "binge" => binge::run(cx, runtime, options).await,
        "warnings" => warnings::run(cx, runtime, options).await,
        _ => Err(WatchError::UnknownSubcommand(name.to_string())),
    }
}

async fn history(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    name: &str,
    options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    match name {
        "streak" => streak::run(cx, options).await,
        "privacy" => privacy::run(cx, options).await,
        "trivia" => trivia::run(cx, runtime, options).await,
        _ => Err(WatchError::UnknownSubcommand(name.to_string())),
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

fn recommend_group() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommandGroup,
        "recommend",
        "Find something to watch",
    )
    .add_sub_option(sub("trending", "What is popular right now"))
    .add_sub_option(
        sub("because", "More like a particular title").add_sub_option(
            string("title", "The title to branch off")
                .required(true)
                .set_autocomplete(true),
        ),
    )
    .add_sub_option(
        sub("mix", "Something between two titles")
            .add_sub_option(
                string("first", "First title").required(true).set_autocomplete(true),
            )
            .add_sub_option(
                string("second", "Second title")
                    .required(true)
                    .set_autocomplete(true),
            ),
    )
    .add_sub_option(sub("for-me", "Based on your own watch history"))
    .add_sub_option(
        sub("hidden-gem", "Well rated, barely watched here")
            .add_sub_option(CreateCommandOption::new(
                CommandOptionType::Number,
                "min_rating",
                "Minimum community rating (default 7.5)",
            ))
            .add_sub_option(string("genre", "Restrict to one genre")),
    )
    .add_sub_option(
        sub("vibe", "Describe a mood and I will translate it").add_sub_option(
            string("description", "For example: a rainy 90s sci-fi with a twist")
                .required(true),
        ),
    )
}

fn party_group() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommandGroup,
        "party",
        "Organise a watch party",
    )
    .add_sub_option(
        sub("create", "Schedule one")
            .add_sub_option(
                string("title", "Something already on the server")
                    .required(true)
                    .set_autocomplete(true),
            )
            .add_sub_option(
                string("when", "When it starts, e.g. 2026-09-14 20:00")
                    .required(true),
            )
            .add_sub_option(CreateCommandOption::new(
                CommandOptionType::Boolean,
                "guests",
                "Offer temporary accounts to people without a login",
            ))
            .add_sub_option(CreateCommandOption::new(
                CommandOptionType::Channel,
                "channel",
                "Where to announce it",
            )),
    )
    .add_sub_option(sub("list", "Show upcoming parties"))
    .add_sub_option(
        sub("info", "Show one party").add_sub_option(
            CreateCommandOption::new(CommandOptionType::Integer, "id", "Party id")
                .required(true),
        ),
    )
    .add_sub_option(
        sub("cancel", "Call one off").add_sub_option(
            CreateCommandOption::new(CommandOptionType::Integer, "id", "Party id")
                .required(true),
        ),
    )
}
