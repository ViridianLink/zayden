mod announce;
mod attachment;
mod build;
mod cradle;
mod faction;
mod map;
mod meta;
mod runner;
mod schedule;
mod weapon;

use serenity::all::{CommandOptionType, CreateCommand, CreateCommandOption};
use zayden_core::{InvocationCtx, parse_options, parse_subcommand};

use crate::client::MarathonClient;
use crate::error::{MarathonError, Result};

pub struct Command;

impl Command {
    pub fn register() -> CreateCommand<'static> {
        let weapon = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "weapon",
            "Base gun stats and attachment slots",
        )
        .add_sub_option(name_option("Weapon name"));

        let attachment = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "attachment",
            "What an attachment does, its slot, and which weapons accept it",
        )
        .add_sub_option(name_option("Attachment name"));

        let runner = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "runner",
            "Shell abilities, cores, and base stats",
        )
        .add_sub_option(name_option("Runner name"));

        let cradle = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "cradle",
            "The Cradle stat system and its nodes",
        );

        let build = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "build",
            "A community build recipe: shell, Cradle focus, and gear",
        )
        .add_sub_option(name_option("Build name"));

        let map = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "map",
            "POIs, possible extractions, event spawns, keycard rooms, and status",
        )
        .add_sub_option(name_option("Map name"));

        let faction = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "faction",
            "Priority contracts and upgrades",
        )
        .add_sub_option(name_option("Faction name"));

        let meta = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "meta",
            "Current dominant weapons / tier snapshot",
        );

        let schedule = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "schedule",
            "Ranked / Cryo Archive windows and the daily duo map pool",
        );

        let announce_set = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "set",
            "Set this server's schedule-announcement channel",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "channel",
                "Channel to post announcements in",
            )
            .required(true),
        );

        let announce_disable = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "disable",
            "Disable schedule announcements for this server",
        );

        let announce = CreateCommandOption::new(
            CommandOptionType::SubCommandGroup,
            "announce",
            "Configure schedule announcements (Manage Server)",
        )
        .add_sub_option(announce_set)
        .add_sub_option(announce_disable);

        CreateCommand::new("marathon")
            .description(
                "Marathon guide: weapons, runners, maps, factions, meta, and schedule",
            )
            .add_option(weapon)
            .add_option(attachment)
            .add_option(runner)
            .add_option(cradle)
            .add_option(build)
            .add_option(map)
            .add_option(faction)
            .add_option(meta)
            .add_option(schedule)
            .add_option(announce)
    }

    pub async fn run(cx: &InvocationCtx<'_>, client: &MarathonClient) -> Result<()> {
        let (name, sub_options) = parse_subcommand(cx.interaction.data.options())
            .map_err(MarathonError::from)?;

        match name {
            "weapon" => weapon::run(cx, client, parse_options(sub_options)).await,
            "attachment" => {
                attachment::run(cx, client, parse_options(sub_options)).await
            },
            "runner" => runner::run(cx, client, parse_options(sub_options)).await,
            "cradle" => cradle::run(cx, client).await,
            "build" => build::run(cx, client, parse_options(sub_options)).await,
            "map" => map::run(cx, client, parse_options(sub_options)).await,
            "faction" => faction::run(cx, client, parse_options(sub_options)).await,
            "meta" => meta::run(cx, client).await,
            "schedule" => schedule::run(cx, client).await,
            "announce" => announce::run(cx, sub_options).await,
            _ => Err(MarathonError::NotFound {
                entity: "subcommand",
                query: name.to_string(),
            }),
        }
    }
}

fn name_option(description: &'static str) -> CreateCommandOption<'static> {
    CreateCommandOption::new(CommandOptionType::String, "name", description)
        .required(true)
        .set_autocomplete(true)
}

pub(super) fn find_entity<'a, T>(
    items: &'a [T],
    query: &str,
    slug: impl Fn(&T) -> &str,
    name: impl Fn(&T) -> &str,
) -> Option<&'a T> {
    if let Some(item) = items.iter().find(|item| slug(item) == query) {
        return Some(item);
    }
    if let Some(item) =
        items.iter().find(|item| name(item).eq_ignore_ascii_case(query))
    {
        return Some(item);
    }
    let query_lower = query.to_lowercase();
    items.iter().find(|item| name(item).to_lowercase().contains(&query_lower))
}
