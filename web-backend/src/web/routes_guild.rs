use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde_json::Value;
use tracing::warn;

use crate::{Result, WebState};

const DISCORD_API: &str = "https://discord.com/api/v10";

async fn discord_get(state: &WebState, path: &str) -> Option<Value> {
    let url = format!("{DISCORD_API}{path}");
    match state
        .app
        .http
        .get(&url)
        .header("Authorization", format!("Bot {}", state.discord_token))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp.json().await.ok(),
        Ok(resp) => {
            warn!(status = %resp.status(), url, "Discord API error");
            None
        },
        Err(e) => {
            warn!(?e, url, "Discord API request failed");
            None
        },
    }
}

pub(super) async fn guild(
    Path(id): Path<String>,
    State(state): State<WebState>,
) -> Result<Json<Option<Value>>> {
    let Ok(guild_id) = id.parse::<u64>() else {
        return Ok(Json(None));
    };
    Ok(Json(
        discord_get(&state, &format!("/guilds/{guild_id}?with_counts=true")).await,
    ))
}

pub(super) async fn channels(
    Path(id): Path<String>,
    State(state): State<WebState>,
) -> Result<Json<Option<Value>>> {
    let Ok(guild_id) = id.parse::<u64>() else {
        return Ok(Json(None));
    };
    Ok(Json(discord_get(&state, &format!("/guilds/{guild_id}/channels")).await))
}

pub(super) async fn zayden(
    Path(guild_id): Path<String>,
    State(state): State<WebState>,
) -> Result<Json<Option<Value>>> {
    let Ok(guild_id) = guild_id.parse::<u64>() else {
        return Ok(Json(None));
    };
    Ok(Json(
        discord_get(&state, &format!("/users/@me/guilds/{guild_id}/member")).await,
    ))
}

pub(super) async fn settings(
    Path(_guild_id): Path<String>,
    State(_state): State<WebState>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
