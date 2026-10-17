mod abilities;
mod build;
mod counter;
mod matchup;
mod summaries;

use deadlock_api::Priority;
use deadlock_api::model::{Hero, Rank};
use deadlock_api::rank::BadgeRange;
use serenity::all::{CommandOptionType, CreateCommand, CreateCommandOption};
use zayden_core::{InvocationCtx, parse_options, parse_subcommand};

use crate::client::DeadlockApi;
use crate::error::{DeadlockError, Result};
use crate::heroes::HeroIndex;

pub struct Command;

impl Command {
    pub fn register() -> CreateCommand<'static> {
        let build = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "build",
            "Recommended item path for a hero",
        )
        .add_sub_option(hero_option("hero", "Hero to look up"))
        .add_sub_option(rank_option());

        let abilities = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "abilities",
            "Ability upgrade order for a hero",
        )
        .add_sub_option(hero_option("hero", "Hero to look up"))
        .add_sub_option(rank_option());

        let counter = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "counter",
            "Best and worst matchups for a hero",
        )
        .add_sub_option(hero_option("hero", "Hero to look up"));

        let matchup = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "matchup",
            "Lane matchup between two heroes",
        )
        .add_sub_option(hero_option("hero", "Your hero"))
        .add_sub_option(hero_option("vs", "Enemy laner"))
        .add_sub_option(rank_option());

        let summaries = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "summaries",
            "Post-match summary preferences",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "dm",
            "DM me a summary after each match",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "server",
            "Share my summaries in this server's summary channel",
        ));

        CreateCommand::new("deadlock")
            .description("Deadlock builds, ability orders and matchups.")
            .add_option(build)
            .add_option(abilities)
            .add_option(counter)
            .add_option(matchup)
            .add_option(summaries)
    }

    pub async fn run(cx: &InvocationCtx<'_>, api: &DeadlockApi) -> Result<()> {
        let (name, sub_options) = parse_subcommand(cx.interaction.data.options())
            .map_err(DeadlockError::from)?;

        match name {
            "build" => build::run(cx, api, parse_options(sub_options)).await,
            "abilities" => abilities::run(cx, api, parse_options(sub_options)).await,
            "counter" => counter::run(cx, api, parse_options(sub_options)).await,
            "matchup" => matchup::run(cx, api, parse_options(sub_options)).await,
            "summaries" => {
                summaries::run(cx, &cx.app.db, parse_options(sub_options)).await
            },
            _ => Err(DeadlockError::InvalidOption(name.to_string())),
        }
    }
}

pub(crate) fn hero_option(
    name: &'static str,
    description: &'static str,
) -> CreateCommandOption<'static> {
    CreateCommandOption::new(CommandOptionType::String, name, description)
        .required(true)
        .set_autocomplete(true)
}

pub(crate) fn rank_option() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::String,
        "rank",
        "Limit stats to an average match rank",
    )
    .set_autocomplete(true)
}

pub(crate) async fn resolve_hero(
    api: &DeadlockApi,
    query: &str,
) -> Result<(Hero, HeroIndex)> {
    let fetched = api.heroes(None, Priority::Interactive).await?;
    let index = HeroIndex::new(&fetched.value);

    let hero = index
        .find(query)
        .cloned()
        .ok_or_else(|| DeadlockError::UnknownHero(query.to_string()))?;

    Ok((hero, index))
}

pub(crate) fn resolve_rank(
    ranks: &[Rank],
    query: &str,
) -> Result<(BadgeRange, String)> {
    let query = query.trim();

    let by_tier = query
        .parse::<u8>()
        .ok()
        .and_then(|tier| ranks.iter().find(|rank| rank.tier == tier));
    let rank = by_tier
        .or_else(|| ranks.iter().find(|rank| rank.name.eq_ignore_ascii_case(query)));

    rank.filter(|rank| rank.tier >= 1)
        .and_then(|rank| {
            BadgeRange::tier(rank.tier).map(|range| (range, rank.name.clone()))
        })
        .ok_or_else(|| DeadlockError::UnknownRank(query.to_string()))
}
