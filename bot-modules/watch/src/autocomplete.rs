use std::sync::Arc;

use jellyfin::index::LibraryItemRow;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{
    AutocompleteChoice,
    CreateAutocompleteResponse,
    CreateInteractionResponse,
    ResolvedValue,
};
use zayden_core::{AutocompleteCtx, parse_subcommand};

use crate::discovery::resolve;
use crate::error::Result;

const CHOICES: i64 = 25;

pub async fn run(
    cx: &AutocompleteCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
) -> Result<()> {
    let Some(focused) = cx.interaction.data.autocomplete() else {
        return Ok(());
    };

    let path = subcommand_path(cx);
    let query = focused.value;

    let choices = match path.as_deref() {
        Some("party create") => local_choices(cx, query, None).await?,
        Some("binge") => local_choices(cx, query, Some("Series")).await?,
        _ => remote_choices(cx, runtime, query).await?,
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

fn subcommand_path(cx: &AutocompleteCtx<'_>) -> Option<String> {
    let (name, inner) = parse_subcommand(cx.interaction.data.options()).ok()?;

    // A group's options contain exactly one nested subcommand; a plain
    // subcommand's do not.
    let nested = inner.iter().find_map(|option| {
        matches!(
            option.value,
            ResolvedValue::SubCommand(_) | ResolvedValue::SubCommandGroup(_)
        )
        .then_some(option.name)
    });

    Some(nested.map_or_else(|| name.to_owned(), |leaf| format!("{name} {leaf}")))
}

async fn local_choices(
    cx: &AutocompleteCtx<'_>,
    query: &str,
    item_type: Option<&str>,
) -> Result<Vec<AutocompleteChoice<'static>>> {
    let rows = LibraryItemRow::search(&cx.app.db, query, item_type, CHOICES).await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            // The value is the Jellyfin item id, so the command skips resolution
            // entirely when the user picks a suggestion.
            AutocompleteChoice::new(
                label(&row.name, row.production_year),
                row.item_id,
            )
        })
        .collect())
}

async fn remote_choices(
    cx: &AutocompleteCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    query: &str,
) -> Result<Vec<AutocompleteChoice<'static>>> {
    if query.trim().len() < 2 {
        return local_choices(cx, query, None).await;
    }

    let results = resolve::search_remote(runtime, query).await?;

    Ok(results
        .into_iter()
        .filter(|r| r.kind().is_some())
        .take(usize::try_from(CHOICES).unwrap_or(25))
        .map(|r| {
            let title = r.display_title().to_owned();
            let year = r.year().and_then(|y| y.parse().ok());
            AutocompleteChoice::new(label(&title, year), title)
        })
        .collect())
}

fn label(name: &str, year: Option<i32>) -> String {
    let base =
        year.map_or_else(|| name.to_owned(), |year| format!("{name} ({year})"));

    if base.chars().count() <= 100 {
        return base;
    }
    base.chars().take(99).chain(std::iter::once('…')).collect()
}
