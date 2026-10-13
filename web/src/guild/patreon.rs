use patreon::{PatreonAnnounceRow, PatreonConnection};
use topcoat::context::Cx;
use zayden_app::state::AppState as ZaydenAppState;

use super::access::admin_app;
use super::announce::{
    PingRoleForm,
    new_ping_role,
    parse_message,
    ping_role_limit,
    refused_ping_role,
};
use super::dto::PatreonStatus;
use super::error::{GuildError, server_err};
use super::form::{GuildForm, form_args};
use super::parse::{parse_flag, parse_role};
use crate::auth::{
    admin_guild_id,
    ensure_bot_can_post,
    ensure_guild_channel,
    web_state,
};

form_args! {
    PatreonSettingsForm { guild, channel_id, public_only, message }
}

/// The guild's Patreon connection and announcement settings.
pub(crate) async fn fetch_patreon_status(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<PatreonStatus, GuildError> {
    let connection =
        PatreonConnection::select(&app.db, guild_id).await.map_err(server_err)?;

    let announce =
        PatreonAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?;

    Ok(PatreonStatus {
        connected: connection.is_some(),
        disabled: connection.as_ref().is_some_and(|c| c.disabled_at.is_some()),
        creator_name: connection.as_ref().and_then(|c| c.creator_name.clone()),
        campaign_id: connection.as_ref().map(|c| c.campaign_id.clone()),
        webhook_registered: connection
            .as_ref()
            .is_some_and(|c| c.webhook_id.is_some()),
        channel_id: announce.as_ref().map(|a| a.channel_id.to_string()),
        public_only: announce.as_ref().is_some_and(|a| a.public_only),
        message: announce.as_ref().and_then(|a| a.message.clone()),
        ping_role_ids: announce
            .iter()
            .flat_map(|a| &a.ping_role_ids)
            .map(ToString::to_string)
            .collect(),
    })
}

pub async fn get_patreon_status(
    cx: &Cx,
    guild: &str,
) -> Result<PatreonStatus, GuildError> {
    tracing::warn!(guild, "get_patreon_status called; it has no known caller");
    let (guild_id, app) = admin_app(cx, guild).await?;
    fetch_patreon_status(app, guild_id).await
}

pub async fn save_patreon_settings(
    cx: &Cx,
    form: &PatreonSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channel_id = form.channel_id.trim();

    if channel_id.is_empty() {
        PatreonAnnounceRow::delete(&app.db, guild_id)
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

    PatreonAnnounceRow::upsert(
        &app.db,
        guild_id,
        channel_id,
        parse_flag(&form.public_only),
        message,
    )
    .await
    .map_err(server_err)
}

pub async fn add_patreon_ping_role(
    cx: &Cx,
    form: &PingRoleForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let Some(announce) =
        PatreonAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?
    else {
        return Err(GuildError::NoAnnounceChannel);
    };

    let role =
        new_ping_role(cx, guild_id, &announce.ping_role_ids, &form.role_id).await?;

    let added = PatreonAnnounceRow::add_ping_role(
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
        PatreonAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?;
    Err(refused_ping_role(
        stored.as_ref().map(|row| row.ping_role_ids.as_slice()),
        role,
    ))
}

pub async fn remove_patreon_ping_role(
    cx: &Cx,
    form: &PingRoleForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let role = parse_role(&form.role_id)?.cast_signed();

    let removed = PatreonAnnounceRow::remove_ping_role(&app.db, guild_id, role)
        .await
        .map_err(server_err)?;

    if removed { Ok(()) } else { Err(GuildError::RoleNotPinged) }
}

pub async fn can_manage_patreon(cx: &Cx, guild: &str) -> Result<bool, GuildError> {
    tracing::warn!(guild, "can_manage_patreon called; it has no known caller");
    admin_guild_id(cx, guild).await.map(|_id| true).map_err(GuildError::from)
}

pub async fn disconnect_patreon(
    cx: &Cx,
    form: &GuildForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let connection =
        PatreonConnection::select(&app.db, guild_id).await.map_err(server_err)?;

    PatreonConnection::delete(&app.db, guild_id).await.map_err(server_err)?;

    if let Some(connection) = connection {
        let patreon_app =
            web_state(cx).ok().and_then(|state| state.integrations.patreon.as_ref());
        patreon::release(&app.http, &app.db, patreon_app, &connection).await;
    }

    Ok(())
}
