use sqlx::PgPool;
use suggestions::ReviewThresholds;
use ticket::{GuildId, HelperLinks, RoleId, SupportRoles, UserId};
use topcoat::context::Cx;
use twilight_http::Client;
use twilight_model::id::Id;

use super::access::admin_app;
use super::dto::HelperLinkInfo;
use super::error::{GuildError, server_err};
use super::form::form_args;
use super::parse::{
    parse_archive_secs,
    parse_flag,
    parse_id,
    parse_idle_secs,
    parse_link,
    parse_role,
    parse_user,
};
use crate::auth::{GuildIds, admin_guild_id, db_pool, discord_client};

form_args! {
    SupportSettingsForm {
        guild,
        support_channel_id,
        solved_tag_id,
        closed_tag_id,
        solved_archive_secs,
    }
}

form_args! {
    IdleSettingsForm {
        guild,
        idle_enabled,
        idle_after_secs,
        idle_close_enabled,
        idle_close_after_secs,
    }
}

form_args! {
    StaleSettingsForm { guild, stale_enabled, stale_tag_id, stale_after_secs }
}

form_args! {
    TicketSettingsForm {
        guild,
        support_channel_id,
        solved_tag_id,
        closed_tag_id,
        solved_archive_secs,
        idle_enabled,
        idle_after_secs,
        idle_close_enabled,
        idle_close_after_secs,
        stale_enabled,
        stale_tag_id,
        stale_after_secs,
    }
}

form_args! {
    SuggestionsSettingsForm {
        guild,
        suggestions_channel_id,
        review_channel_id,
        promote_threshold,
        demote_threshold,
    }
}

form_args! {
    SupportRoleForm { guild, role_id }
}

form_args! {
    AddHelperLinkForm { guild, user_id, link }
}

form_args! {
    RemoveHelperLinkForm { guild, user_id }
}

const DEFAULT_IDLE_SECS: i32 = 172_800;
const DEFAULT_IDLE_CLOSE_SECS: i32 = 86_400;
const DEFAULT_STALE_SECS: i32 = 604_800;

const fn ticket_guild(guild_id: i64) -> GuildId {
    GuildId::new(guild_id.cast_unsigned())
}

pub async fn save_support_settings(
    cx: &Cx,
    form: &SupportSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let archive_secs = parse_archive_secs(&form.solved_archive_secs);
    let support_channel_id = parse_id(&form.support_channel_id);

    GuildIds::default().channel(support_channel_id).ensure_in(cx, guild_id).await?;

    app.settings
        .support
        .update(guild_id, |p| {
            p.support_channel_id = support_channel_id;
            p.solved_tag_id = parse_id(&form.solved_tag_id);
            p.closed_tag_id = parse_id(&form.closed_tag_id);
            p.solved_archive_secs = archive_secs;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn save_idle_settings(
    cx: &Cx,
    form: &IdleSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let enabled = parse_flag(&form.idle_enabled);
    let idle_secs = parse_idle_secs(&form.idle_after_secs, DEFAULT_IDLE_SECS);
    let close_enabled = parse_flag(&form.idle_close_enabled);
    let close_secs =
        parse_idle_secs(&form.idle_close_after_secs, DEFAULT_IDLE_CLOSE_SECS);

    app.settings
        .support
        .update(guild_id, |p| {
            p.idle_enabled = enabled;
            p.idle_after_secs = idle_secs;
            p.idle_close_enabled = close_enabled;
            p.idle_close_after_secs = close_secs;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn save_stale_settings(
    cx: &Cx,
    form: &StaleSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let enabled = parse_flag(&form.stale_enabled);
    let tag = parse_id(&form.stale_tag_id);
    let secs = parse_idle_secs(&form.stale_after_secs, DEFAULT_STALE_SECS);

    app.settings
        .support
        .update(guild_id, |p| {
            p.stale_enabled = enabled;
            p.stale_tag_id = tag;
            p.stale_after_secs = secs;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

/// Saves the ticket channel, tags, idle reminders and stale marking together:
/// the channel is checked against the server, then one row write applies
/// every field.
pub async fn save_ticket_settings(
    cx: &Cx,
    form: &TicketSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let support_channel_id = parse_id(&form.support_channel_id);
    let solved_tag_id = parse_id(&form.solved_tag_id);
    let closed_tag_id = parse_id(&form.closed_tag_id);
    let archive_secs = parse_archive_secs(&form.solved_archive_secs);
    let idle_enabled = parse_flag(&form.idle_enabled);
    let idle_secs = parse_idle_secs(&form.idle_after_secs, DEFAULT_IDLE_SECS);
    let close_enabled = parse_flag(&form.idle_close_enabled);
    let close_secs =
        parse_idle_secs(&form.idle_close_after_secs, DEFAULT_IDLE_CLOSE_SECS);
    let stale_enabled = parse_flag(&form.stale_enabled);
    let stale_tag_id = parse_id(&form.stale_tag_id);
    let stale_secs = parse_idle_secs(&form.stale_after_secs, DEFAULT_STALE_SECS);

    GuildIds::default().channel(support_channel_id).ensure_in(cx, guild_id).await?;

    app.settings
        .support
        .update(guild_id, |p| {
            p.support_channel_id = support_channel_id;
            p.solved_tag_id = solved_tag_id;
            p.closed_tag_id = closed_tag_id;
            p.solved_archive_secs = archive_secs;
            p.idle_enabled = idle_enabled;
            p.idle_after_secs = idle_secs;
            p.idle_close_enabled = close_enabled;
            p.idle_close_after_secs = close_secs;
            p.stale_enabled = stale_enabled;
            p.stale_tag_id = stale_tag_id;
            p.stale_after_secs = stale_secs;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn save_suggestions_settings(
    cx: &Cx,
    form: &SuggestionsSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let thresholds =
        ReviewThresholds::parse(&form.promote_threshold, &form.demote_threshold);
    let suggestions_channel_id = parse_id(&form.suggestions_channel_id);
    let review_channel_id = parse_id(&form.review_channel_id);

    GuildIds::default()
        .channel(suggestions_channel_id)
        .channel(review_channel_id)
        .ensure_in(cx, guild_id)
        .await?;

    app.settings
        .suggestions
        .update(guild_id, |p| {
            p.suggestions_channel_id = suggestions_channel_id;
            p.review_channel_id = review_channel_id;
            p.promote_threshold = thresholds.promote();
            p.demote_threshold = thresholds.demote();
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub(crate) async fn fetch_support_roles(
    pool: &PgPool,
    guild_id: i64,
) -> Result<Vec<String>, GuildError> {
    Ok(SupportRoles::ids(pool, ticket_guild(guild_id))
        .await?
        .into_iter()
        .map(|id| id.get().to_string())
        .collect())
}

pub async fn list_support_roles(
    cx: &Cx,
    guild: &str,
) -> Result<Vec<String>, GuildError> {
    let guild_id = admin_guild_id(cx, guild).await?;
    let pool = db_pool(cx)?;
    fetch_support_roles(pool, guild_id).await
}

pub async fn add_support_role(
    cx: &Cx,
    form: &SupportRoleForm,
) -> Result<(), GuildError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let pool = db_pool(cx)?;

    let role = parse_role(&form.role_id)?;

    GuildIds::default()
        .role(Some(role.cast_signed()))
        .ensure_in(cx, guild_id)
        .await?;

    let added =
        SupportRoles::add(pool, ticket_guild(guild_id), RoleId::new(role)).await?;

    if added { Ok(()) } else { Err(GuildError::DuplicateSupportRole) }
}

pub async fn remove_support_role(
    cx: &Cx,
    form: &SupportRoleForm,
) -> Result<(), GuildError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let pool = db_pool(cx)?;

    let role = parse_role(&form.role_id)?;

    SupportRoles::remove(pool, ticket_guild(guild_id), RoleId::new(role)).await?;

    Ok(())
}

pub(crate) async fn fetch_helper_links(
    pool: &PgPool,
    http: &Client,
    guild_id: i64,
) -> Result<Vec<HelperLinkInfo>, GuildError> {
    let links = HelperLinks::list(pool, ticket_guild(guild_id)).await?;

    let mut out = Vec::with_capacity(links.len());

    for l in links {
        let user_id = l.user_id.get();
        let name = display_name(http, guild_id.cast_unsigned(), user_id).await;

        out.push(HelperLinkInfo {
            user_id: user_id.to_string(),
            name,
            link: l.link,
        });
    }

    Ok(out)
}

pub async fn list_helper_links(
    cx: &Cx,
    guild: &str,
) -> Result<Vec<HelperLinkInfo>, GuildError> {
    let guild_id = admin_guild_id(cx, guild).await?;
    let pool = db_pool(cx)?;
    let http = discord_client(cx)?;
    fetch_helper_links(pool, http, guild_id).await
}

async fn display_name(http: &Client, guild_id: u64, user_id: u64) -> String {
    let member = async {
        let resp = http
            .guild_member(Id::new_checked(guild_id)?, Id::new_checked(user_id)?)
            .await
            .ok()?;
        resp.model().await.ok()
    }
    .await;

    member.map_or_else(
        || format!("unknown ({user_id})"),
        |m| m.nick.unwrap_or_else(|| m.user.global_name.unwrap_or(m.user.name)),
    )
}

pub async fn add_helper_link(
    cx: &Cx,
    form: &AddHelperLinkForm,
) -> Result<(), GuildError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let pool = db_pool(cx)?;

    let user = parse_user(&form.user_id)?;
    let link = parse_link(&form.link)?;

    HelperLinks::set(pool, ticket_guild(guild_id), UserId::new(user), &link)
        .await
        .map_err(server_err)
}

pub async fn remove_helper_link(
    cx: &Cx,
    form: &RemoveHelperLinkForm,
) -> Result<(), GuildError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let pool = db_pool(cx)?;

    let user = parse_user(&form.user_id)?;

    HelperLinks::remove(pool, ticket_guild(guild_id), UserId::new(user)).await?;

    Ok(())
}
