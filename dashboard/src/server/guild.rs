use leptos::prelude::*;

use crate::dto::{GuildInfo, GuildSettings};

#[server]
pub async fn list_manageable_guilds() -> Result<Vec<GuildInfo>, ServerFnError> {
    use sqlx::{PgPool, Row};
    use tower_cookies::Cookies;
    use twilight_model::guild::Permissions;

    use crate::server::auth::{bearer_client, server_err};

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
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

    let all_guilds = bearer_client(&access_token)
        .current_user_guilds()
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;

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

    use zayden_app::state::AppState;

    use crate::server::auth::{guild_admin_context, server_err};

    fn opt_str(v: Option<i64>) -> Option<String> {
        v.map(|n| n.to_string())
    }

    let (guild_id_i64, _user_id, _access_token) =
        guild_admin_context(&guild_id).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    let support =
        app.settings.support.get(guild_id_i64).await.map_err(server_err)?;
    let suggestions =
        app.settings.suggestions.get(guild_id_i64).await.map_err(server_err)?;
    let channels =
        app.settings.channels.get(guild_id_i64).await.map_err(server_err)?;
    let roles = app.settings.roles.get(guild_id_i64).await.map_err(server_err)?;
    let temp_voice =
        app.settings.temp_voice.get(guild_id_i64).await.map_err(server_err)?;
    let lfg = app.settings.lfg.get(guild_id_i64).await.map_err(server_err)?;

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
    })
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

    use crate::server::auth::{guild_admin_context, server_err};

    let (guild_id_i64, _user_id, _access_token) =
        guild_admin_context(&guild).await?;

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

    use crate::server::auth::{guild_admin_context, server_err};

    let (guild_id_i64, _user_id, _access_token) =
        guild_admin_context(&guild).await?;

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

    use crate::server::auth::{guild_admin_context, server_err};

    let (guild_id_i64, _user_id, _access_token) =
        guild_admin_context(&guild).await?;

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

    use crate::server::auth::{guild_admin_context, server_err};

    let (guild_id_i64, _user_id, _access_token) =
        guild_admin_context(&guild).await?;

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

    use crate::server::auth::{guild_admin_context, server_err};

    let (guild_id_i64, _user_id, _access_token) =
        guild_admin_context(&guild).await?;

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
