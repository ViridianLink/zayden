use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{
        app_state,
        bearer_client,
        db_pool,
        discord_client,
        guild_admin_context,
        server_err,
    },
    leptos_axum::{extract, redirect},
    std::sync::Arc,
    suggestions::ReviewThresholds,
    ticket::{GuildId, RoleId, SupportRoles},
    tower_cookies::Cookies,
    twilight_model::channel::ChannelType,
    twilight_model::guild::Permissions,
    twilight_model::id::Id,
    zayden_app::config::MusicSettingsRow,
    zayden_app::state::AppState,
};

use crate::dto::{GuildInfo, GuildSettings};

#[cfg(feature = "ssr")]
async fn admin_app(guild: &str) -> Result<(i64, Arc<AppState>), ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(guild).await?;
    Ok((guild_id, app_state()?))
}

#[cfg(feature = "ssr")]
fn parse_id(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.is_empty() { None } else { t.parse().ok() }
}

#[cfg(feature = "ssr")]
fn parse_role(s: &str) -> Result<RoleId, ServerFnError> {
    s.trim()
        .parse::<u64>()
        .map(RoleId::new)
        .map_err(|_e| ServerFnError::ServerError("invalid role".to_string()))
}

#[server]
pub async fn list_manageable_guilds() -> Result<Vec<GuildInfo>, ServerFnError> {
    let pool = db_pool()?;

    let cookies: Cookies = extract().await.map_err(server_err)?;
    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        redirect("/login");
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let row = sqlx::query_scalar!(
        "SELECT discord_access_token FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
        &token,
    )
    .fetch_optional(&pool)
    .await
    .map_err(server_err)?;
    let Some(access_token) = row else {
        redirect("/login");
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let all_guilds = bearer_client(&access_token)
        .current_user_guilds()
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;

    Ok(all_guilds
        .into_iter()
        .filter(|g| {
            g.permissions
                .intersects(Permissions::ADMINISTRATOR | Permissions::MANAGE_GUILD)
        })
        .map(|g| GuildInfo {
            id: g.id.to_string(),
            name: g.name,
            icon: g.icon.map(|hash| hash.to_string()),
        })
        .collect())
}

#[server]
pub async fn get_guild_settings(
    guild_id: String,
) -> Result<GuildSettings, ServerFnError> {
    fn opt_str(v: Option<i64>) -> Option<String> {
        v.map(|n| n.to_string())
    }

    let (guild_id, app) = admin_app(&guild_id).await?;
    let s = &app.settings;

    let support = s.support.get(guild_id).await.map_err(server_err)?;
    let suggestions = s.suggestions.get(guild_id).await.map_err(server_err)?;
    let channels = s.channels.get(guild_id).await.map_err(server_err)?;
    let roles = s.roles.get(guild_id).await.map_err(server_err)?;
    let temp_voice = s.temp_voice.get(guild_id).await.map_err(server_err)?;
    let lfg = s.lfg.get(guild_id).await.map_err(server_err)?;
    let family = s.family.get(guild_id).await.map_err(server_err)?;
    let music = s.music.get(guild_id).await.map_err(server_err)?;
    let honeypot = s.honeypot.get(guild_id).await.map_err(server_err)?;

    Ok(GuildSettings {
        support_channel_id: opt_str(support.support_channel_id),
        faq_channel_id: opt_str(support.faq_channel_id),
        suggestions_channel_id: opt_str(suggestions.suggestions_channel_id),
        review_channel_id: opt_str(suggestions.review_channel_id),
        suggestions_promote_threshold: suggestions.promote_threshold.to_string(),
        suggestions_demote_threshold: suggestions.demote_threshold.to_string(),
        rules_channel_id: opt_str(channels.rules_channel_id),
        general_channel_id: opt_str(channels.general_channel_id),
        spoiler_channel_id: opt_str(channels.spoiler_channel_id),
        artist_role_id: opt_str(roles.artist_role_id),
        sleep_role_id: opt_str(roles.sleep_role_id),
        temp_voice_category: opt_str(temp_voice.temp_voice_category),
        temp_voice_creator_channel: opt_str(temp_voice.temp_voice_creator_channel),
        lfg_channel_id: opt_str(lfg.lfg_channel_id),
        lfg_role_id: opt_str(lfg.lfg_role_id),
        lfg_scheduled_thread_id: opt_str(lfg.lfg_scheduled_thread_id),
        family_max_partners: family.max_partners.to_string(),
        music_dj_role_id: opt_str(music.dj_role_id),
        music_auto_disconnect_secs: music.auto_disconnect_secs.to_string(),
        music_announce_now_playing: music.announce_now_playing,
        music_announce_channel_id: opt_str(music.announce_channel_id),
        honeypot_channel_id: opt_str(honeypot.channel_id),
        honeypot_exempt_admins: honeypot.exempt_admins,
        honeypot_exempt_role_id: opt_str(honeypot.exempt_role_id),
    })
}

#[server]
pub async fn save_support_settings(
    guild: String,
    support_channel_id: String,
    faq_channel_id: String,
    suggestions_channel_id: String,
    review_channel_id: String,
    promote_threshold: String,
    demote_threshold: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let thresholds = ReviewThresholds::parse(&promote_threshold, &demote_threshold);

    app.settings
        .support
        .update(guild_id, |p| {
            p.support_channel_id = parse_id(&support_channel_id);
            p.faq_channel_id = parse_id(&faq_channel_id);
        })
        .await
        .map_err(server_err)?;

    app.settings
        .suggestions
        .update(guild_id, |p| {
            p.suggestions_channel_id = parse_id(&suggestions_channel_id);
            p.review_channel_id = parse_id(&review_channel_id);
            p.promote_threshold = thresholds.promote();
            p.demote_threshold = thresholds.demote();
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn list_support_roles(
    guild: String,
) -> Result<Vec<String>, ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(&guild).await?;
    let pool = db_pool()?;

    Ok(SupportRoles::ids(&pool, GuildId::new(guild_id.cast_unsigned()))
        .await
        .map_err(server_err)?
        .into_iter()
        .map(|id| id.get().to_string())
        .collect())
}

#[server]
pub async fn add_support_role(
    guild: String,
    role_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(&guild).await?;
    let pool = db_pool()?;

    let role = parse_role(&role_id)?;

    let added =
        SupportRoles::add(&pool, GuildId::new(guild_id.cast_unsigned()), role)
            .await
            .map_err(server_err)?;

    if added {
        Ok(())
    } else {
        Err(ServerFnError::ServerError(
            "that role is already a support role".to_string(),
        ))
    }
}

#[server]
pub async fn remove_support_role(
    guild: String,
    role_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(&guild).await?;
    let pool = db_pool()?;

    let role = parse_role(&role_id)?;

    SupportRoles::remove(&pool, GuildId::new(guild_id.cast_unsigned()), role)
        .await
        .map_err(server_err)?;

    Ok(())
}

#[server]
pub async fn save_channel_settings(
    guild: String,
    rules_channel_id: String,
    general_channel_id: String,
    spoiler_channel_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    app.settings
        .channels
        .update(guild_id, |p| {
            p.rules_channel_id = parse_id(&rules_channel_id);
            p.general_channel_id = parse_id(&general_channel_id);
            p.spoiler_channel_id = parse_id(&spoiler_channel_id);
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_role_settings(
    guild: String,
    artist_role_id: String,
    sleep_role_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    app.settings
        .roles
        .update(guild_id, |p| {
            p.artist_role_id = parse_id(&artist_role_id);
            p.sleep_role_id = parse_id(&sleep_role_id);
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_temp_voice_settings(
    guild: String,
    temp_voice_category: String,
    temp_voice_creator_channel: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    app.settings
        .temp_voice
        .update(guild_id, |p| {
            p.temp_voice_category = parse_id(&temp_voice_category);
            p.temp_voice_creator_channel = parse_id(&temp_voice_creator_channel);
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[cfg(feature = "ssr")]
const CREATOR_CHANNEL_NAME: &str = "\u{2795} Creator Channel";

#[server]
pub async fn create_temp_voice_creator_channel(
    guild: String,
    temp_voice_category: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let category = parse_id(&temp_voice_category)
        .and_then(|id| Id::new_checked(id.cast_unsigned()));
    let Some(category) = category else {
        return Err(ServerFnError::ServerError(
            "select a category first".to_string(),
        ));
    };

    let channel = discord_client()?
        .create_guild_channel(
            Id::new(guild_id.cast_unsigned()),
            CREATOR_CHANNEL_NAME,
        )
        .kind(ChannelType::GuildVoice)
        .parent_id(category)
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;

    app.settings
        .temp_voice
        .update(guild_id, |p| {
            p.temp_voice_category = Some(category.get().cast_signed());
            p.temp_voice_creator_channel = Some(channel.id.get().cast_signed());
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_family_settings(
    guild: String,
    max_partners: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let max_partners = max_partners.trim().parse::<i32>().unwrap_or(1).max(1);

    app.settings
        .family
        .update(guild_id, |p| {
            p.max_partners = max_partners;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_music_settings(
    guild: String,
    dj_role_id: String,
    auto_disconnect_secs: String,
    announce_now_playing: String,
    announce_channel_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let auto_disconnect_secs =
        MusicSettingsRow::parse_auto_disconnect_secs(&auto_disconnect_secs);
    let announce_now_playing = announce_now_playing.trim() == "true";

    app.settings
        .music
        .update(guild_id, |p| {
            p.dj_role_id = parse_id(&dj_role_id);
            p.auto_disconnect_secs = auto_disconnect_secs;
            p.announce_now_playing = announce_now_playing;
            p.announce_channel_id = parse_id(&announce_channel_id);
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_honeypot_settings(
    guild: String,
    channel_id: String,
    exempt_admins: String,
    exempt_role_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let exempt_admins = exempt_admins.trim() == "true";

    app.settings
        .honeypot
        .update(guild_id, |p| {
            p.channel_id = parse_id(&channel_id);
            p.exempt_admins = exempt_admins;
            p.exempt_role_id = parse_id(&exempt_role_id);
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_lfg_settings(
    guild: String,
    lfg_channel_id: String,
    lfg_role_id: String,
    lfg_scheduled_thread_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    app.settings
        .lfg
        .update(guild_id, |p| {
            p.lfg_channel_id = parse_id(&lfg_channel_id);
            p.lfg_role_id = parse_id(&lfg_role_id);
            p.lfg_scheduled_thread_id = parse_id(&lfg_scheduled_thread_id);
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}
