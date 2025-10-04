use std::env;
use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use twilight_http::Client;
use twilight_model::channel::Channel;
use twilight_model::guild::Guild;
use twilight_model::id::Id;

use crate::{AppState, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct GuildResponse(Guild);

pub async fn guild(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Option<GuildResponse>>> {
    // TODO: Recieve & validate user auth-token

    let guild_id = Id::new(id.parse().unwrap());

    match state.discord_client.guild(guild_id).with_counts(true).await {
        Ok(response) => {
            let guild = response.model().await.unwrap();
            Ok(Json(Some(GuildResponse(guild))))
        }
        Err(e) => Ok(Json(None)),
    }
}

pub async fn channels(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Option<Vec<Channel>>>> {
    let guild_id = Id::new(id.parse().unwrap());

    match state.discord_client.guild_channels(guild_id).await {
        Ok(response) => {
            let guild = response.model().await.unwrap();
            Ok(Json(Some(guild)))
        }
        Err(e) => Ok(Json(None)),
    }
}
