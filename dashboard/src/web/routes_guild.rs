use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use tracing::warn;

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

fn opt_str(v: Option<i64>) -> Option<String> {
    v.map(|n| n.to_string())
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

    let settings = &state.app.settings;
    let support = settings.support.get(guild_id).await?;
    let suggestions = settings.suggestions.get(guild_id).await?;
    let channels = settings.channels.get(guild_id).await?;
    let roles = settings.roles.get(guild_id).await?;
    let temp_voice = settings.temp_voice.get(guild_id).await?;
    let lfg = settings.lfg.get(guild_id).await?;

    Ok(Json(serde_json::json!({
        "id": guild_id.to_string(),
        "support_channel_id": opt_str(support.support_channel_id),
        "support_role_id": opt_str(support.support_role_id),
        "faq_channel_id": opt_str(support.faq_channel_id),
        "suggestions_channel_id": opt_str(suggestions.suggestions_channel_id),
        "review_channel_id": opt_str(suggestions.review_channel_id),
        "rules_channel_id": opt_str(channels.rules_channel_id),
        "general_channel_id": opt_str(channels.general_channel_id),
        "spoiler_channel_id": opt_str(channels.spoiler_channel_id),
        "artist_role_id": opt_str(roles.artist_role_id),
        "sleep_role_id": opt_str(roles.sleep_role_id),
        "temp_voice_category": opt_str(temp_voice.temp_voice_category),
        "temp_voice_creator_channel": opt_str(temp_voice.temp_voice_creator_channel),
        "lfg_channel_id": opt_str(lfg.lfg_channel_id),
        "lfg_role_id": opt_str(lfg.lfg_role_id),
        "lfg_scheduled_thread_id": opt_str(lfg.lfg_scheduled_thread_id),
    })))
}

pub(super) async fn settings_patch(
    Path(id): Path<String>,
    State(state): State<WebState>,
    Json(body): Json<SettingsPatch>,
) -> Result<Json<Value>> {
    let Ok(guild_id) = id.parse::<i64>() else {
        return Err(Error::BadRequest);
    };

    let settings = &state.app.settings;

    let support = settings
        .support
        .update(guild_id, |p| {
            p.support_channel_id = body.support_channel_id;
            p.support_role_id = body.support_role_id;
            p.faq_channel_id = body.faq_channel_id;
        })
        .await?;
    let suggestions = settings
        .suggestions
        .update(guild_id, |p| {
            p.suggestions_channel_id = body.suggestions_channel_id;
            p.review_channel_id = body.review_channel_id;
        })
        .await?;
    let channels = settings
        .channels
        .update(guild_id, |p| {
            p.rules_channel_id = body.rules_channel_id;
            p.general_channel_id = body.general_channel_id;
            p.spoiler_channel_id = body.spoiler_channel_id;
        })
        .await?;
    let roles = settings
        .roles
        .update(guild_id, |p| {
            p.artist_role_id = body.artist_role_id;
            p.sleep_role_id = body.sleep_role_id;
        })
        .await?;
    let temp_voice = settings
        .temp_voice
        .update(guild_id, |p| {
            p.temp_voice_category = body.temp_voice_category;
            p.temp_voice_creator_channel = body.temp_voice_creator_channel;
        })
        .await?;
    let lfg = settings
        .lfg
        .update(guild_id, |p| {
            p.lfg_channel_id = body.lfg_channel_id;
            p.lfg_role_id = body.lfg_role_id;
            p.lfg_scheduled_thread_id = body.lfg_scheduled_thread_id;
        })
        .await?;

    Ok(Json(serde_json::json!({
        "id": guild_id.to_string(),
        "support_channel_id": opt_str(support.support_channel_id),
        "support_role_id": opt_str(support.support_role_id),
        "faq_channel_id": opt_str(support.faq_channel_id),
        "suggestions_channel_id": opt_str(suggestions.suggestions_channel_id),
        "review_channel_id": opt_str(suggestions.review_channel_id),
        "rules_channel_id": opt_str(channels.rules_channel_id),
        "general_channel_id": opt_str(channels.general_channel_id),
        "spoiler_channel_id": opt_str(channels.spoiler_channel_id),
        "artist_role_id": opt_str(roles.artist_role_id),
        "sleep_role_id": opt_str(roles.sleep_role_id),
        "temp_voice_category": opt_str(temp_voice.temp_voice_category),
        "temp_voice_creator_channel": opt_str(temp_voice.temp_voice_creator_channel),
        "lfg_channel_id": opt_str(lfg.lfg_channel_id),
        "lfg_role_id": opt_str(lfg.lfg_role_id),
        "lfg_scheduled_thread_id": opt_str(lfg.lfg_scheduled_thread_id),
    })))
}
