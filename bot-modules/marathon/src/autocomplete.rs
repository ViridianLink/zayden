use serenity::all::{
    AutocompleteChoice,
    CreateAutocompleteResponse,
    CreateInteractionResponse,
    ResolvedValue,
};
use zayden_core::{AutocompleteCtx, parse_subcommand};

use crate::client::MarathonClient;
use crate::error::{MarathonError, Result};

pub async fn run(cx: &AutocompleteCtx<'_>, client: &MarathonClient) -> Result<()> {
    let (name, sub_options) = parse_subcommand(cx.interaction.data.options())
        .map_err(MarathonError::from)?;

    let query = sub_options
        .iter()
        .find_map(|option| match option.value {
            ResolvedValue::Autocomplete { value, .. } => Some(value),
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
        })
        .unwrap_or_default();
    let query_lower = query.to_lowercase();

    let choices: Vec<AutocompleteChoice<'static>> = match name {
        "weapon" => client.weapons().await.map_or_default(|items| {
            filter_choices(&items, &query_lower, |w| (&w.slug, &w.name))
        }),
        "attachment" => client.attachments().await.map_or_default(|items| {
            filter_choices(&items, &query_lower, |a| (&a.slug, &a.name))
        }),
        "runner" => client.runners().await.map_or_default(|items| {
            filter_choices(&items, &query_lower, |r| (&r.slug, &r.name))
        }),
        "build" => client.builds().await.map_or_default(|items| {
            filter_choices(&items, &query_lower, |b| (&b.slug, &b.name))
        }),
        "map" => client.maps().await.map_or_default(|items| {
            filter_choices(&items, &query_lower, |m| (&m.slug, &m.name))
        }),
        "faction" => client.factions().await.map_or_default(|items| {
            filter_choices(&items, &query_lower, |f| (&f.slug, &f.name))
        }),
        _ => Vec::new(),
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

fn filter_choices<T>(
    items: &[T],
    query_lower: &str,
    fields: impl Fn(&T) -> (&str, &str),
) -> Vec<AutocompleteChoice<'static>> {
    items
        .iter()
        .filter_map(|item| {
            let (slug, name) = fields(item);
            name.to_lowercase()
                .contains(query_lower)
                .then(|| AutocompleteChoice::new(name.to_string(), slug.to_string()))
        })
        .take(25)
        .collect()
}
