pub mod about;
pub mod link;
pub mod privacy;
pub mod streak;
pub mod unlink;

use std::sync::Arc;

use serenity::all::{CommandOptionType, CreateCommand, CreateCommandOption};
use zayden_core::{InvocationCtx, parse_options, parse_subcommand};

use crate::error::{JellyfinError, Result};
use crate::runtime::JellyfinRuntime;

pub struct Jellyfin;

impl Jellyfin {
    pub fn register() -> CreateCommand<'static> {
        CreateCommand::new("jellyfin")
            .description("Link your Jellyfin account and see your watch stats")
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "link",
                "Link your Jellyfin account with a one-time code (no password)",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "unlink",
                "Forget the Jellyfin account linked to your Discord",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "streak",
                    "Show consecutive days watched",
                )
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "Whose streak to show (defaults to you)",
                )),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "privacy",
                    "Choose whether others can see your watch streak",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Boolean,
                        "visible",
                        "True to let others run /jellyfin streak on you",
                    )
                    .required(true),
                ),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "about",
                "Show what is on the media server",
            ))
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
            "streak" => streak::run(cx, options).await,
            "privacy" => privacy::run(cx, options).await,
            "about" => about::run(cx, runtime).await,
            _ => Err(JellyfinError::UnknownSubcommand(name.to_string())),
        }
    }
}
