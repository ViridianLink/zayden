use std::sync::Arc;
use std::time::Duration;

use serenity::all::{
    AutocompleteChoice,
    CreateAutocompleteResponse,
    CreateInteractionResponse,
    ResolvedValue,
};
use zayden_core::{AutocompleteCtx, as_i64, parse_subcommand};

use crate::client::{PalworldClient, SourceKey};
use crate::error::{PalworldError, Result};
use crate::model::PlayerName;
use crate::upload::SaveUpload;

const CHOICE_BUDGET: Duration = Duration::from_millis(2_500);

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

    let host = (name == "link")
        .then(|| {
            sub_options.iter().find_map(|option| {
                if option.name != "host" {
                    return None;
                }
                if let ResolvedValue::User(user, _) = option.value {
                    Some(as_i64(user.id.get()))
                } else {
                    None
                }
            })
        })
        .flatten();

    let build = async {
        if is_player_field {
            let discord_id = as_i64(cx.interaction.user.id.get());
            return player_names(client, &cx.app.db, discord_id, host)
                .await
                .map(|players| {
                    players
                        .iter()
                        .filter(|p| p.search_key.contains(&query_lower))
                        .take(25)
                        .map(|p| {
                            AutocompleteChoice::new(p.name.clone(), p.name.clone())
                        })
                        .collect()
                })
                .unwrap_or_default();
        }

        match name {
            "pal" | "breeding" | "breed-for" | "breed-plan" => client
                .pals_basic()
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

    let choices =
        tokio::time::timeout(CHOICE_BUDGET, build).await.unwrap_or_else(|_| {
            tracing::warn!(
                command = name,
                field = focused_name,
                "palworld: autocomplete exceeded its budget; answering empty",
            );
            Vec::new()
        });

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

async fn player_names(
    client: &PalworldClient,
    pool: &sqlx::PgPool,
    discord_id: i64,
    host: Option<i64>,
) -> Result<Arc<[PlayerName]>> {
    if let Some(host) = host
        && let Some(upload) = SaveUpload::select(pool, host).await?
        && !upload.is_expired()
    {
        return client.player_names(SourceKey::User(host)).await;
    }
    if let Some(upload) = SaveUpload::select(pool, discord_id).await?
        && !upload.is_expired()
    {
        return client.player_names(SourceKey::User(discord_id)).await;
    }
    client.player_names(SourceKey::Shared).await
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
