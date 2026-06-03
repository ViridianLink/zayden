use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use tracing::warn;
use zayden_app::config::guild_config::{GuildConfig, GuildConfigPatch};

use crate::{Error, Result, WebState};

const DISCORD_API: &str = "https://discord.com/api/v10";

async fn discord_get(state: &WebState, path: &str) -> Result<Value> {
    let url = format!("{DISCORD_API}{path}");
    let resp = state
        .app
        .http
        .get(&url)
        .header("Authorization", format!("Bot {}", state.discord_token))
        .send()
        .await
        .map_err(|e| Error::Upstream(e.to_string()))?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Err(Error::NotFound);
    }

    if !resp.status().is_success() {
        warn!(status = %resp.status(), url, "Discord API error");
        return Err(Error::Upstream(format!("Discord returned {}", resp.status())));
    }

    resp.json().await.map_err(|e| Error::Upstream(e.to_string()))
}

fn guild_config_to_json(cfg: &GuildConfig) -> Value {
    let updated_at_ms = cfg.updated_at.to_jiff().as_millisecond();
    serde_json::json!({
        "id": cfg.id.to_string(),
        "support_channel_id": cfg.support_channel_id.map(|v| v.to_string()),
        "support_role_id": cfg.support_role_id.map(|v| v.to_string()),
        "faq_channel_id": cfg.faq_channel_id.map(|v| v.to_string()),
        "suggestions_channel_id": cfg.suggestions_channel_id.map(|v| v.to_string()),
        "review_channel_id": cfg.review_channel_id.map(|v| v.to_string()),
        "rules_channel_id": cfg.rules_channel_id.map(|v| v.to_string()),
        "general_channel_id": cfg.general_channel_id.map(|v| v.to_string()),
        "spoiler_channel_id": cfg.spoiler_channel_id.map(|v| v.to_string()),
        "artist_role_id": cfg.artist_role_id.map(|v| v.to_string()),
        "sleep_role_id": cfg.sleep_role_id.map(|v| v.to_string()),
        "temp_voice_category": cfg.temp_voice_category.map(|v| v.to_string()),
        "temp_voice_creator_channel": cfg.temp_voice_creator_channel.map(|v| v.to_string()),
        "lfg_channel_id": cfg.lfg_channel_id.map(|v| v.to_string()),
        "lfg_role_id": cfg.lfg_role_id.map(|v| v.to_string()),
        "lfg_scheduled_thread_id": cfg.lfg_scheduled_thread_id.map(|v| v.to_string()),
        "updated_at": updated_at_ms,
    })
}

pub(super) async fn guild(
    Path(id): Path<String>,
    State(state): State<WebState>,
) -> Result<Json<Value>> {
    let Ok(guild_id) = id.parse::<u64>() else {
        return Err(Error::BadRequest);
    };

    let json =
        discord_get(&state, &format!("/guilds/{guild_id}?with_counts=true")).await?;

    Ok(Json(json))
}

pub(super) async fn channels(
    Path(id): Path<String>,
    State(state): State<WebState>,
) -> Result<Json<Value>> {
    let Ok(guild_id) = id.parse::<u64>() else {
        return Err(Error::BadRequest);
    };

    Ok(Json(discord_get(&state, &format!("/guilds/{guild_id}/channels")).await?))
}

pub(super) async fn zayden(
    Path(guild_id): Path<String>,
    State(state): State<WebState>,
) -> Result<Json<Value>> {
    let Ok(guild_id) = guild_id.parse::<u64>() else {
        return Err(Error::BadRequest);
    };

    Ok(Json(
        discord_get(&state, &format!("/users/@me/guilds/{guild_id}/member")).await?,
    ))
}

#[derive(Deserialize)]
pub(super) struct SettingsPatch {
    support_channel_id: Option<i64>,
    support_role_id: Option<i64>,
    faq_channel_id: Option<i64>,
    suggestions_channel_id: Option<i64>,
    review_channel_id: Option<i64>,
    rules_channel_id: Option<i64>,
    general_channel_id: Option<i64>,
    spoiler_channel_id: Option<i64>,
    artist_role_id: Option<i64>,
    sleep_role_id: Option<i64>,
    temp_voice_category: Option<i64>,
    temp_voice_creator_channel: Option<i64>,
    lfg_channel_id: Option<i64>,
    lfg_role_id: Option<i64>,
    lfg_scheduled_thread_id: Option<i64>,
}

pub(super) async fn settings(
    Path(id): Path<String>,
    State(state): State<WebState>,
) -> Result<Json<Value>> {
    let Ok(guild_id) = id.parse::<i64>() else {
        return Err(Error::BadRequest);
    };

    let cfg = state.app.config_store.try_get(guild_id).await?;

    let json = cfg.map_or_else(
        || guild_config_to_json(&GuildConfig::empty(guild_id)),
        |c| guild_config_to_json(&c),
    );

    Ok(Json(json))
}

pub(super) async fn settings_patch(
    Path(id): Path<String>,
    State(state): State<WebState>,
    Json(body): Json<SettingsPatch>,
) -> Result<Json<Value>> {
    let Ok(guild_id) = id.parse::<i64>() else {
        return Err(Error::BadRequest);
    };

    let updated = state
        .app
        .config_store
        .update(guild_id, |p: &mut GuildConfigPatch| {
            p.support_channel_id = body.support_channel_id;
            p.support_role_id = body.support_role_id;
            p.faq_channel_id = body.faq_channel_id;
            p.suggestions_channel_id = body.suggestions_channel_id;
            p.review_channel_id = body.review_channel_id;
            p.rules_channel_id = body.rules_channel_id;
            p.general_channel_id = body.general_channel_id;
            p.spoiler_channel_id = body.spoiler_channel_id;
            p.artist_role_id = body.artist_role_id;
            p.sleep_role_id = body.sleep_role_id;
            p.temp_voice_category = body.temp_voice_category;
            p.temp_voice_creator_channel = body.temp_voice_creator_channel;
            p.lfg_channel_id = body.lfg_channel_id;
            p.lfg_role_id = body.lfg_role_id;
            p.lfg_scheduled_thread_id = body.lfg_scheduled_thread_id;
        })
        .await?;

    Ok(Json(guild_config_to_json(&updated)))
}
