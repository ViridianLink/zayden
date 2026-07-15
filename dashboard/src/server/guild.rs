use leptos::prelude::*;

use crate::dto::{GuildInfo, GuildSettings};

#[server]
pub async fn list_manageable_guilds() -> Result<Vec<GuildInfo>, ServerFnError> {
    use reqwest::Client;
    use sqlx::{PgPool, Row};
    use tower_cookies::Cookies;
    use twilight_model::guild::Permissions;
    use twilight_model::user::CurrentUserGuild;

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };
    let Some(http) = use_context::<Client>() else {
        return Err(ServerFnError::ServerError("missing HTTP client".to_string()));
    };

    let cookies: Cookies = match leptos_axum::extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        leptos_axum::redirect("/login");
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let access_token: String = match sqlx::query(
        "SELECT discord_access_token FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(r)) => r.get("discord_access_token"),
        Ok(None) => {
            leptos_axum::redirect("/login");
            return Err(ServerFnError::ServerError("unauthenticated".to_string()));
        },
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let resp = match http
        .get("https://discord.com/api/v10/users/@me/guilds")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    if !resp.status().is_success() {
        return Err(ServerFnError::ServerError(format!(
            "Discord API returned {}",
            resp.status()
        )));
    }

    let all_guilds: Vec<CurrentUserGuild> = match resp.json().await {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let guilds = all_guilds
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
        .collect();

    Ok(guilds)
}

#[server]
pub async fn get_guild_settings(
    guild_id: String,
) -> Result<GuildSettings, ServerFnError> {
    use std::sync::Arc;

    use reqwest::Client;
    use sqlx::PgPool;
    use tower_cookies::Cookies;
    use twilight_model::guild::Permissions;
    use twilight_model::user::CurrentUserGuild;
    use zayden_app::entitlement::types::{EntitlementScope, Tier};
    use zayden_app::state::AppState;

    let Ok(guild_id_i64) = guild_id.parse::<i64>() else {
        return Err(ServerFnError::ServerError("invalid guild id".to_string()));
    };
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };
    let Some(http) = use_context::<Client>() else {
        return Err(ServerFnError::ServerError("missing HTTP client".to_string()));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    let cookies: Cookies = match leptos_axum::extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        leptos_axum::redirect("/login");
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let row = match sqlx::query(
        "SELECT discord_access_token, discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let Some(row) = row else {
        leptos_axum::redirect("/login");
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };
    use sqlx::Row as _;
    let access_token: String = row.get("discord_access_token");
    let discord_user_id: i64 = row.get("discord_user_id");
    let discord_user_id_u64 = discord_user_id.cast_unsigned();

    let guilds_resp = match http
        .get("https://discord.com/api/v10/users/@me/guilds")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    if !guilds_resp.status().is_success() {
        return Err(ServerFnError::ServerError(
            "failed to fetch guild list from Discord".to_string(),
        ));
    }
    let all_guilds: Vec<CurrentUserGuild> = match guilds_resp.json().await {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let has_access = all_guilds.iter().any(|g| {
        g.id.get() == guild_id_u64
            && g.permissions
                .intersects(Permissions::ADMINISTRATOR | Permissions::MANAGE_GUILD)
    });
    if !has_access {
        return Err(ServerFnError::ServerError("forbidden".to_string()));
    }

    fn opt_str(v: Option<i64>) -> Option<String> {
        v.map(|n| n.to_string())
    }
    fn app_err(e: sqlx::Error) -> ServerFnError {
        ServerFnError::ServerError(e.to_string())
    }

    let support = app.settings.support.get(guild_id_i64).await.map_err(app_err)?;
    let suggestions =
        app.settings.suggestions.get(guild_id_i64).await.map_err(app_err)?;
    let channels = app.settings.channels.get(guild_id_i64).await.map_err(app_err)?;
    let roles = app.settings.roles.get(guild_id_i64).await.map_err(app_err)?;
    let temp_voice =
        app.settings.temp_voice.get(guild_id_i64).await.map_err(app_err)?;
    let lfg = app.settings.lfg.get(guild_id_i64).await.map_err(app_err)?;

    let scope = EntitlementScope::UserInGuild(discord_user_id_u64, guild_id_u64);
    let is_pro = app.entitlements.allows(scope, Tier::Pro).await;

    Ok(GuildSettings {
        support_channel_id: opt_str(support.support_channel_id),
        support_role_id: opt_str(support.support_role_id),
        faq_channel_id: opt_str(support.faq_channel_id),
        suggestions_channel_id: opt_str(suggestions.suggestions_channel_id),
        review_channel_id: opt_str(suggestions.review_channel_id),
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
        is_pro,
    })
}

#[cfg(feature = "ssr")]
async fn guild_write_guard(guild_id_str: &str) -> Result<(i64, u64), ServerFnError> {
    use std::sync::Arc;

    use reqwest::Client;
    use sqlx::PgPool;
    use tower_cookies::Cookies;
    use twilight_model::guild::Permissions;
    use twilight_model::user::CurrentUserGuild;
    use zayden_app::entitlement::types::{EntitlementScope, Tier};
    use zayden_app::state::AppState;

    let Ok(guild_id_i64) = guild_id_str.parse::<i64>() else {
        return Err(ServerFnError::ServerError("invalid guild id".to_string()));
    };
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };
    let Some(http) = use_context::<Client>() else {
        return Err(ServerFnError::ServerError("missing HTTP client".to_string()));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    let cookies: Cookies = match leptos_axum::extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let row = match sqlx::query(
        "SELECT discord_access_token, discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let Some(row) = row else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };
    use sqlx::Row as _;
    let access_token: String = row.get("discord_access_token");
    let discord_user_id: i64 = row.get("discord_user_id");
    let discord_user_id_u64 = discord_user_id.cast_unsigned();

    let guilds_resp = match http
        .get("https://discord.com/api/v10/users/@me/guilds")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    if !guilds_resp.status().is_success() {
        return Err(ServerFnError::ServerError(
            "failed to fetch guild list from Discord".to_string(),
        ));
    }
    let all_guilds: Vec<CurrentUserGuild> = match guilds_resp.json().await {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let has_access = all_guilds.iter().any(|g| {
        g.id.get() == guild_id_u64
            && g.permissions
                .intersects(Permissions::ADMINISTRATOR | Permissions::MANAGE_GUILD)
    });
    if !has_access {
        return Err(ServerFnError::ServerError("forbidden".to_string()));
    }

    let scope = EntitlementScope::UserInGuild(discord_user_id_u64, guild_id_u64);
    if !app.entitlements.allows(scope, Tier::Pro).await {
        return Err(ServerFnError::ServerError("pro_required".to_string()));
    }

    Ok((guild_id_i64, discord_user_id_u64))
}

#[cfg(feature = "ssr")]
fn parse_id(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.is_empty() { None } else { t.parse().ok() }
}

#[server]
pub async fn save_support_settings(
    guild: String,
    support_channel_id: String,
    support_role_id: String,
    faq_channel_id: String,
    suggestions_channel_id: String,
    review_channel_id: String,
) -> Result<(), ServerFnError> {
    use std::sync::Arc;

    use zayden_app::state::AppState;

    use crate::server::auth::server_err;

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .support
        .update(guild_id_i64, |p| {
            p.support_channel_id = parse_id(&support_channel_id);
            p.support_role_id = parse_id(&support_role_id);
            p.faq_channel_id = parse_id(&faq_channel_id);
        })
        .await
        .map_err(server_err)?;

    app.settings
        .suggestions
        .update(guild_id_i64, |p| {
            p.suggestions_channel_id = parse_id(&suggestions_channel_id);
            p.review_channel_id = parse_id(&review_channel_id);
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn save_channel_settings(
    guild: String,
    rules_channel_id: String,
    general_channel_id: String,
    spoiler_channel_id: String,
) -> Result<(), ServerFnError> {
    use std::sync::Arc;

    use zayden_app::state::AppState;

    use crate::server::auth::server_err;

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .channels
        .update(guild_id_i64, |p| {
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
    use std::sync::Arc;

    use zayden_app::state::AppState;

    use crate::server::auth::server_err;

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .roles
        .update(guild_id_i64, |p| {
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
    use std::sync::Arc;

    use zayden_app::state::AppState;

    use crate::server::auth::server_err;

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .temp_voice
        .update(guild_id_i64, |p| {
            p.temp_voice_category = parse_id(&temp_voice_category);
            p.temp_voice_creator_channel = parse_id(&temp_voice_creator_channel);
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
    use std::sync::Arc;

    use zayden_app::state::AppState;

    use crate::server::auth::server_err;

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .lfg
        .update(guild_id_i64, |p| {
            p.lfg_channel_id = parse_id(&lfg_channel_id);
            p.lfg_role_id = parse_id(&lfg_role_id);
            p.lfg_scheduled_thread_id = parse_id(&lfg_scheduled_thread_id);
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}
