use serenity::all::{
    AutocompleteChoice,
    CreateAutocompleteResponse,
    CreateInteractionResponse,
    ResolvedValue,
};
use zayden_core::{AutocompleteCtx, parse_subcommand};

use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};

pub async fn run(cx: &AutocompleteCtx<'_>, client: &PalworldClient) -> Result<()> {
    let (name, sub_options) = parse_subcommand(cx.interaction.data.options())
        .map_err(PalworldError::from)?;

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
    let (focused_name, query) = focused.unwrap_or_default();
    let query_lower = query.to_lowercase();

    let is_player_field = matches!(
        (name, focused_name),
        ("roster" | "breed-plan", "player") | ("link", "name")
    );

    let choices: Vec<AutocompleteChoice<'static>> = if is_player_field {
        client
            .roster()
            .await
            .map(|roster| {
                roster
                    .players
                    .iter()
                    .filter(|p| p.name.to_lowercase().contains(&query_lower))
                    .take(25)
                    .map(|p| AutocompleteChoice::new(p.name.clone(), p.name.clone()))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        match name {
            "pal" | "breeding" | "breed-for" | "breed-plan" => client
                .pals()
                .await
                .map(|items| {
                    filter_choices(items.iter(), &query_lower, |p| (&p.key, &p.name))
                })
                .unwrap_or_default(),
            "item" => client
                .items()
                .await
                .map(|items| {
                    filter_choices(items.iter(), &query_lower, |i| (&i.key, &i.name))
                })
                .unwrap_or_default(),
            "passive" => client
                .passives()
                .await
                .map(|items| {
                    filter_choices(items.iter(), &query_lower, |p| (&p.key, &p.name))
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
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

fn filter_choices<'a, T: 'a>(
    items: impl Iterator<Item = &'a T>,
    query_lower: &str,
    fields: impl Fn(&'a T) -> (&'a str, &'a str),
) -> Vec<AutocompleteChoice<'static>> {
    items
        .filter_map(|item| {
            let (key, name) = fields(item);
            name.to_lowercase()
                .contains(query_lower)
                .then(|| AutocompleteChoice::new(name.to_string(), key.to_string()))
        })
        .take(25)
        .collect()
}
