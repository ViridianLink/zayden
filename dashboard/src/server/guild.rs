use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{
        admin_guild_id,
        app_state,
        bearer_client,
        db_pool,
        discord_client,
        server_err,
    },
    honeypot::{HoneypotConfig, HoneypotSettings},
    leptos_axum::{extract, redirect},
    std::sync::Arc,
    suggestions::ReviewThresholds,
    ticket::{GuildId, HelperLinks, RoleId, SupportRoles, UserId},
    tower_cookies::Cookies,
    twilight_http::Client,
    twilight_model::channel::ChannelType,
    twilight_model::guild::Permissions,
    twilight_model::id::Id,
    url::Url,
    zayden_app::config::{ARCHIVE_NEVER, MusicSettingsRow},
    zayden_app::state::AppState,
};

use crate::dto::{GuildInfo, GuildSettings, HelperLinkInfo};

#[cfg(feature = "ssr")]
pub(crate) async fn admin_app(
    guild: &str,
) -> Result<(i64, Arc<AppState>), ServerFnError> {
    let guild_id = admin_guild_id(guild).await?;
    Ok((guild_id, app_state()?))
}

#[cfg(feature = "ssr")]
fn parse_id(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.is_empty() { None } else { t.parse().ok() }
}

#[cfg(feature = "ssr")]
fn parse_optional(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_owned()) }
}

#[cfg(feature = "ssr")]
fn parse_wiki_url(s: &str) -> Result<Option<String>, ServerFnError> {
    let Some(raw) = parse_optional(s) else {
        return Ok(None);
    };

    let Ok(url) = Url::parse(raw.trim_end_matches('/')) else {
        return Err(ServerFnError::ServerError("invalid wiki URL".to_string()));
    };

    if !matches!(url.scheme(), "http" | "https") {
        return Err(ServerFnError::ServerError(
            "the wiki URL must start with http:// or https://".to_string(),
        ));
    }

    Ok(Some(url.as_str().trim_end_matches('/').to_owned()))
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
    let ai = s.ai.get(guild_id).await.map_err(server_err)?;
    let faq = s.faq.get(guild_id).await.map_err(server_err)?;

    Ok(GuildSettings {
        support_channel_id: opt_str(support.support_channel_id),
        solved_tag_id: opt_str(support.solved_tag_id),
        closed_tag_id: opt_str(support.closed_tag_id),
        helper_role_id: opt_str(support.helper_role_id),
        solved_archive_secs: support.solved_archive_secs.to_string(),
        suggestions_channel_id: opt_str(suggestions.suggestions_channel_id),
        review_channel_id: opt_str(suggestions.review_channel_id),
        suggestions_promote_threshold: suggestions.promote_threshold.to_string(),
        suggestions_demote_threshold: suggestions.demote_threshold.to_string(),
        rules_channel_id: opt_str(channels.rules_channel_id),
        general_channel_id: opt_str(channels.general_channel_id),
        spoiler_channel_id: opt_str(channels.spoiler_channel_id),
        artist_role_id: opt_str(roles.artist_role_id),
        sleep_role_id: opt_str(roles.sleep_role_id),
        verified_role_id: opt_str(roles.verified_role_id),
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
        honeypot_purge_seconds: honeypot.purge_seconds.to_string(),
        ai_enabled: ai.enabled,
        ai_channel_id: opt_str(ai.channel_id),
        faq_enabled: faq.enabled,
        faq_auto_triage: faq.auto_triage,
        faq_auto_generate: faq.auto_generate,
        faq_wiki_url: faq.wiki_url.clone().unwrap_or_default(),
        faq_wiki_api_key: faq.wiki_api_key.clone().unwrap_or_default(),
        faq_wiki_locale: faq.wiki_locale.clone(),
        faq_max_results: faq.max_results.to_string(),
        faq_answer_max_tokens: faq.answer_max_tokens.to_string(),
        faq_answer_temperature: faq.answer_temperature.to_string(),
    })
}

#[server]
pub async fn save_faq_settings(
    guild: String,
    enabled: String,
    auto_triage: String,
    auto_generate: String,
    wiki_url: String,
    wiki_api_key: String,
    wiki_locale: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let enabled = enabled.trim() == "true";
    let auto_triage = auto_triage.trim() == "true";
    let auto_generate = auto_generate.trim() == "true";
    let url = parse_wiki_url(&wiki_url)?;
    let key = parse_optional(&wiki_api_key);
    let locale = match wiki_locale.trim() {
        "" => String::from("en"),
        locale => locale.to_owned(),
    };

    app.settings
        .faq
        .update(guild_id, |p| {
            p.enabled = enabled;
            p.auto_triage = auto_triage;
            p.auto_generate = auto_generate;
            p.wiki_url = url;
            p.wiki_api_key = key;
            p.wiki_locale = locale;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_faq_tuning(
    guild: String,
    max_results: String,
    answer_max_tokens: String,
    answer_temperature: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let max_results = max_results.trim().parse().unwrap_or(5_i32).clamp(1, 25);
    let max_tokens =
        answer_max_tokens.trim().parse().unwrap_or(500_i32).clamp(64, 4096);
    let temperature =
        answer_temperature.trim().parse().unwrap_or(0.2_f32).clamp(0.0, 2.0);

    app.settings
        .faq
        .update(guild_id, |p| {
            p.max_results = max_results;
            p.answer_max_tokens = max_tokens;
            p.answer_temperature = temperature;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_support_settings(
    guild: String,
    support_channel_id: String,
    solved_tag_id: String,
    closed_tag_id: String,
    helper_role_id: String,
    solved_archive_secs: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let archive_secs = parse_archive_secs(&solved_archive_secs);

    app.settings
        .support
        .update(guild_id, |p| {
            p.support_channel_id = parse_id(&support_channel_id);
            p.solved_tag_id = parse_id(&solved_tag_id);
            p.closed_tag_id = parse_id(&closed_tag_id);
            p.helper_role_id = parse_id(&helper_role_id);
            p.solved_archive_secs = archive_secs;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_suggestions_settings(
    guild: String,
    suggestions_channel_id: String,
    review_channel_id: String,
    promote_threshold: String,
    demote_threshold: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let thresholds = ReviewThresholds::parse(&promote_threshold, &demote_threshold);

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
    let guild_id = admin_guild_id(&guild).await?;
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
    let guild_id = admin_guild_id(&guild).await?;
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
    let guild_id = admin_guild_id(&guild).await?;
    let pool = db_pool()?;

    let role = parse_role(&role_id)?;

    SupportRoles::remove(&pool, GuildId::new(guild_id.cast_unsigned()), role)
        .await
        .map_err(server_err)?;

    Ok(())
}

#[cfg(feature = "ssr")]
fn parse_archive_secs(s: &str) -> i32 {
    // -1 disables archiving; anything else clamps to a non-negative delay.
    match s.trim().parse::<i32>() {
        Ok(n) if n < 0 => ARCHIVE_NEVER,
        Ok(n) => n,
        Err(_e) => 60,
    }
}

#[cfg(feature = "ssr")]
fn parse_user(s: &str) -> Result<UserId, ServerFnError> {
    s.trim()
        .parse::<u64>()
        .map(UserId::new)
        .map_err(|_e| ServerFnError::ServerError("invalid user id".to_string()))
}

#[cfg(feature = "ssr")]
fn parse_link(s: &str) -> Result<String, ServerFnError> {
    let url = match Url::parse(s.trim()) {
        Ok(url) => url,
        Err(e) => {
            return Err(ServerFnError::ServerError(format!("invalid link: {e}")));
        },
    };

    if !matches!(url.scheme(), "http" | "https") {
        return Err(ServerFnError::ServerError(
            "link must be an http:// or https:// address".to_string(),
        ));
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(ServerFnError::ServerError(
            "link must not embed credentials".to_string(),
        ));
    }

    let link = url.to_string();

    if link.len() > 200 {
        return Err(ServerFnError::ServerError("link is too long".to_string()));
    }

    Ok(link)
}

#[server]
pub async fn list_helper_links(
    guild: String,
) -> Result<Vec<HelperLinkInfo>, ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
    let pool = db_pool()?;
    let http = discord_client()?;

    let links = HelperLinks::list(&pool, GuildId::new(guild_id.cast_unsigned()))
        .await
        .map_err(server_err)?;

    let mut out = Vec::with_capacity(links.len());

    for l in links {
        let user_id = l.user_id.get();
        let name = display_name(&http, guild_id.cast_unsigned(), user_id).await;

        out.push(HelperLinkInfo {
            user_id: user_id.to_string(),
            name,
            link: l.link,
        });
    }

    Ok(out)
}

#[cfg(feature = "ssr")]
async fn display_name(http: &Client, guild_id: u64, user_id: u64) -> String {
    let member = async {
        let resp =
            http.guild_member(Id::new(guild_id), Id::new(user_id)).await.ok()?;
        resp.model().await.ok()
    }
    .await;

    member.map_or_else(
        || format!("unknown ({user_id})"),
        |m| m.nick.unwrap_or_else(|| m.user.global_name.unwrap_or(m.user.name)),
    )
}

#[server]
pub async fn add_helper_link(
    guild: String,
    user_id: String,
    link: String,
) -> Result<(), ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
    let pool = db_pool()?;

    let user = parse_user(&user_id)?;
    let link = parse_link(&link)?;

    HelperLinks::set(&pool, GuildId::new(guild_id.cast_unsigned()), user, &link)
        .await
        .map_err(server_err)
}

#[server]
pub async fn remove_helper_link(
    guild: String,
    user_id: String,
) -> Result<(), ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
    let pool = db_pool()?;

    let user = parse_user(&user_id)?;

    HelperLinks::remove(&pool, GuildId::new(guild_id.cast_unsigned()), user)
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
    verified_role_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    app.settings
        .roles
        .update(guild_id, |p| {
            p.artist_role_id = parse_id(&artist_role_id);
            p.sleep_role_id = parse_id(&sleep_role_id);
            p.verified_role_id = parse_id(&verified_role_id);
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
    purge_seconds: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let config = HoneypotConfig::from_form(
        &channel_id,
        exempt_admins.trim() == "true",
        &exempt_role_id,
        &purge_seconds,
    )
    .map_err(server_err)?;

    HoneypotSettings::save(
        &app.settings.honeypot,
        GuildId::new(guild_id.cast_unsigned()),
        config,
    )
    .await
    .map(|_| ())
    .map_err(server_err)
}

#[server]
pub async fn save_ai_settings(
    guild: String,
    enabled: String,
    channel_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let enabled = enabled.trim() == "true";
    let channel_id = parse_id(&channel_id);

    app.settings
        .ai
        .update(guild_id, |p| {
            p.enabled = enabled;
            p.channel_id = channel_id;
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
