use deadlock_api::Priority;
use deadlock_api::model::Rank;
use deadlock_api::rank::MAX_TIER;
use serenity::all::{
    AutocompleteChoice,
    CreateAutocompleteResponse,
    CreateInteractionResponse,
    ResolvedValue,
};
use zayden_core::limits::autocomplete::CHOICES as MAX_CHOICES;
use zayden_core::{AutocompleteCtx, parse_subcommand};

use crate::client::DeadlockApi;
use crate::error::{DeadlockError, Result};
use crate::heroes::HeroIndex;

#[must_use]
pub fn hero_choices(
    index: &HeroIndex,
    query: &str,
) -> Vec<AutocompleteChoice<'static>> {
    index
        .search(query, MAX_CHOICES)
        .into_iter()
        .map(|hero| AutocompleteChoice::new(hero.name.clone(), hero.id.to_string()))
        .collect()
}

#[must_use]
pub fn rank_choices(
    ranks: &[Rank],
    query: &str,
) -> Vec<AutocompleteChoice<'static>> {
    let needle = query.to_lowercase();

    let mut matching: Vec<&Rank> = ranks
        .iter()
        .filter(|rank| {
            (1..=MAX_TIER).contains(&rank.tier)
                && rank.name.to_lowercase().contains(&needle)
        })
        .collect();
    matching.sort_by_key(|rank| rank.tier);

    matching
        .into_iter()
        .take(MAX_CHOICES)
        .map(|rank| {
            AutocompleteChoice::new(rank.name.clone(), rank.tier.to_string())
        })
        .collect()
}

pub async fn run(cx: &AutocompleteCtx<'_>, api: &DeadlockApi) -> Result<()> {
    let (_, sub_options) = parse_subcommand(cx.interaction.data.options())
        .map_err(DeadlockError::from)?;

    let focused = sub_options.iter().find_map(|option| match option.value {
        ResolvedValue::Autocomplete { value, .. } => Some((option.name, value)),
        ResolvedValue::Boolean(_)
        | ResolvedValue::Integer(_)
        | ResolvedValue::Number(_)
        | ResolvedValue::String(_)
        | ResolvedValue::SubCommand(_)
        | ResolvedValue::SubCommandGroup(_)
        | ResolvedValue::Attachment(_)
        | ResolvedValue::Channel(_)
        | ResolvedValue::Role(_)
        | ResolvedValue::User(..)
        | ResolvedValue::Unresolved(_)
        | _ => None,
    });

    let choices: Vec<AutocompleteChoice<'static>> = match focused {
        Some(("hero" | "vs", query)) => {
            api.heroes(None, Priority::Interactive).await.map_or_else(
                |error| {
                    tracing::debug!(%error, "hero autocomplete fetch failed");
                    Vec::new()
                },
                |fetched| hero_choices(&HeroIndex::new(&fetched.value), query),
            )
        },
        Some(("rank", query)) => api.ranks(Priority::Interactive).await.map_or_else(
            |error| {
                tracing::debug!(%error, "rank autocomplete fetch failed");
                Vec::new()
            },
            |fetched| rank_choices(&fetched.value, query),
        ),
        Some(_) | None => Vec::new(),
    };

    cx.interaction
        .create_response(
            &cx.ctx.http,
            CreateInteractionResponse::Autocomplete(
                CreateAutocompleteResponse::new().set_choices(choices),
            ),
        )
        .await?;

    Ok(())
}
