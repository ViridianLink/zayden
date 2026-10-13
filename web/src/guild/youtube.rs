use topcoat::context::Cx;
use youtube::{YoutubeAnnounceRow, YoutubeConnection};
use zayden_app::state::AppState as ZaydenAppState;

use super::access::admin_app;
use super::announce::{
    PingRoleForm,
    new_ping_role,
    parse_message,
    ping_role_limit,
    refused_ping_role,
};
use super::dto::YoutubeStatus;
use super::error::{GuildError, server_err};
use super::form::{GuildForm, form_args};
use super::parse::parse_role;
use crate::auth::{ensure_bot_can_post, ensure_guild_channel, web_state};

form_args! {
    YoutubeSettingsForm { guild, channel_id, message }
}

pub(crate) async fn fetch_youtube_status(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<YoutubeStatus, GuildError> {
    let connection =
        YoutubeConnection::select(&app.db, guild_id).await.map_err(server_err)?;

    let announce =
        YoutubeAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?;

    Ok(YoutubeStatus {
        connected: connection.is_some(),
        channel_title: connection.as_ref().map(|c| c.channel_title.clone()),
        push_active: connection.as_ref().is_some_and(|c| c.lease_active),
        channel_id: announce.as_ref().map(|a| a.channel_id.to_string()),
        message: announce.as_ref().and_then(|a| a.message.clone()),
        ping_role_ids: announce
            .iter()
            .flat_map(|a| &a.ping_role_ids)
            .map(ToString::to_string)
            .collect(),
    })
}

pub async fn save_youtube_settings(
    cx: &Cx,
    form: &YoutubeSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channel_id = form.channel_id.trim();

    if channel_id.is_empty() {
        YoutubeAnnounceRow::delete(&app.db, guild_id)
            .await
            .map(|_removed| ())
            .map_err(server_err)?;

        return Ok(());
    }

    let Ok(channel_id) = channel_id.parse::<i64>() else {
        return Err(GuildError::InvalidChannelId);
    };
    let message = parse_message(&form.message)?;

    ensure_guild_channel(cx, guild_id, channel_id).await?;
    ensure_bot_can_post(cx, guild_id, channel_id).await?;

    YoutubeAnnounceRow::upsert(&app.db, guild_id, channel_id, message)
        .await
        .map_err(server_err)
}

pub async fn add_youtube_ping_role(
    cx: &Cx,
    form: &PingRoleForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let Some(announce) =
        YoutubeAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?
    else {
        return Err(GuildError::NoAnnounceChannel);
    };

    let role =
        new_ping_role(cx, guild_id, &announce.ping_role_ids, &form.role_id).await?;

    let added = YoutubeAnnounceRow::add_ping_role(
        &app.db,
        guild_id,
        role,
        ping_role_limit(),
    )
    .await
    .map_err(server_err)?;
    if added {
        return Ok(());
    }

    let stored =
        YoutubeAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?;
    Err(refused_ping_role(
        stored.as_ref().map(|row| row.ping_role_ids.as_slice()),
        role,
    ))
}

pub async fn remove_youtube_ping_role(
    cx: &Cx,
    form: &PingRoleForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let role = parse_role(&form.role_id)?.cast_signed();

    let removed = YoutubeAnnounceRow::remove_ping_role(&app.db, guild_id, role)
        .await
        .map_err(server_err)?;

    if removed { Ok(()) } else { Err(GuildError::RoleNotPinged) }
}

pub async fn disconnect_youtube(
    cx: &Cx,
    form: &GuildForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let Some(channel_id) =
        YoutubeConnection::delete(&app.db, guild_id).await.map_err(server_err)?
    else {
        return Ok(());
    };

    let runtime = web_state(cx)
        .ok()
        .and_then(|state| state.integrations.youtube_runtime.as_ref());
    youtube::release_channel(
        &app.http,
        &app.db,
        runtime.map(|r| &*r.webhook_uri),
        &channel_id,
    )
    .await;

    Ok(())
}
