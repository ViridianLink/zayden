use serenity::all::{
    AutocompleteChoice,
    CreateAutocompleteResponse,
    CreateInteractionResponse,
    ResolvedValue,
};
use zayden_app::config::RadioStation;
use zayden_core::{AutocompleteCtx, parse_subcommand};

use crate::error::{MusicError, Result};

const MAX_CHOICES: usize = 25;

#[must_use]
pub fn matching_stations<'a>(
    stations: &'a [RadioStation],
    query: &str,
) -> Vec<&'a RadioStation> {
    stations
        .iter()
        .filter(|station| station.matches(query))
        .take(MAX_CHOICES)
        .collect()
}

fn choice_label(station: &RadioStation) -> String {
    station.genre.as_ref().map_or_else(
        || station.name.clone(),
        |genre| format!("{} ({genre})", station.name),
    )
}

pub async fn run(cx: &AutocompleteCtx<'_>) -> Result<()> {
    let (group, sub_options) =
        parse_subcommand(cx.interaction.data.options()).map_err(MusicError::from)?;
    if group != "radio" {
        return Ok(());
    }

    let (name, play_options) =
        parse_subcommand(sub_options).map_err(MusicError::from)?;
    if name != "play" {
        return Ok(());
    }

    let query = play_options
        .iter()
        .find_map(|option| {
            if option.name != "station" {
                return None;
            }
            if let ResolvedValue::Autocomplete { value, .. } = option.value {
                Some(value)
            } else {
                None
            }
        })
        .unwrap_or_default();

    let choices = matching_stations(&cx.app.radio_stations, query)
        .into_iter()
        .map(|station| {
            AutocompleteChoice::new(choice_label(station), station.id.clone())
        })
        .collect::<Vec<_>>();

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
