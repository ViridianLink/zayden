use deadlock::db::announce::PatchAnnounceRow;
use deadlock::db::{squad_channel, summary_channel};
use topcoat::context::Cx;

use super::access::admin_app;
use super::announce::{
    PingRoleForm,
    new_ping_role,
    parse_message,
    ping_role_limit,
    refused_ping_role,
};
use super::error::{GuildError, server_err};
use super::form::form_args;
use super::parse::{parse_flag, parse_role};
use crate::auth::{ensure_bot_can_post, ensure_guild_channel};

form_args! {
    DeadlockSettingsForm { guild, channel_id, message }
}

form_args! {
    DeadlockSummariesForm { guild, summary_channel_id }
}

form_args! {
    DeadlockSquadForm { guild, squad_channel_id, squad_leaderboard }
}

pub async fn save_deadlock_settings(
    cx: &Cx,
    form: &DeadlockSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channel_id = form.channel_id.trim();

    if channel_id.is_empty() {
        PatchAnnounceRow::delete(&app.db, guild_id)
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

    PatchAnnounceRow::upsert(&app.db, guild_id, channel_id, message)
        .await
        .map_err(server_err)
}

pub async fn save_deadlock_summaries(
    cx: &Cx,
    form: &DeadlockSummariesForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channel_id = form.summary_channel_id.trim();

    if channel_id.is_empty() {
        summary_channel::delete(&app.db, guild_id)
            .await
            .map(|_removed| ())
            .map_err(server_err)?;

        return Ok(());
    }

    let Ok(channel_id) = channel_id.parse::<i64>() else {
        return Err(GuildError::InvalidChannelId);
    };

    ensure_guild_channel(cx, guild_id, channel_id).await?;
    ensure_bot_can_post(cx, guild_id, channel_id).await?;

    summary_channel::upsert(&app.db, guild_id, channel_id).await.map_err(server_err)
}

pub async fn save_deadlock_squad(
    cx: &Cx,
    form: &DeadlockSquadForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channel_id = form.squad_channel_id.trim();

    if channel_id.is_empty() {
        squad_channel::delete(&app.db, guild_id)
            .await
            .map(|_removed| ())
            .map_err(server_err)?;

        return Ok(());
    }

    let Ok(channel_id) = channel_id.parse::<i64>() else {
        return Err(GuildError::InvalidChannelId);
    };

    ensure_guild_channel(cx, guild_id, channel_id).await?;
    ensure_bot_can_post(cx, guild_id, channel_id).await?;

    squad_channel::upsert(
        &app.db,
        guild_id,
        channel_id,
        parse_flag(&form.squad_leaderboard),
    )
    .await
    .map_err(server_err)
}

pub async fn add_deadlock_ping_role(
    cx: &Cx,
    form: &PingRoleForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let Some(announce) =
        PatchAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?
    else {
        return Err(GuildError::NoAnnounceChannel);
    };

    let role =
        new_ping_role(cx, guild_id, &announce.ping_role_ids, &form.role_id).await?;

    let added =
        PatchAnnounceRow::add_ping_role(&app.db, guild_id, role, ping_role_limit())
            .await
            .map_err(server_err)?;
    if added {
        return Ok(());
    }

    let stored =
        PatchAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?;
    Err(refused_ping_role(
        stored.as_ref().map(|row| row.ping_role_ids.as_slice()),
        role,
    ))
}

pub async fn remove_deadlock_ping_role(
    cx: &Cx,
    form: &PingRoleForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let role = parse_role(&form.role_id)?.cast_signed();

    let removed = PatchAnnounceRow::remove_ping_role(&app.db, guild_id, role)
        .await
        .map_err(server_err)?;

    if removed { Ok(()) } else { Err(GuildError::RoleNotPinged) }
}
