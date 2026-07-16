use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    leptos_axum::extract,
    sqlx::{PgPool, Row},
    std::sync::Arc,
    tower_cookies::Cookies,
    twilight_http::Client,
    twilight_model::guild::Permissions,
    zayden_app::state::AppState,
};

#[cfg(feature = "ssr")]
pub(crate) fn server_err<E: std::fmt::Display>(e: E) -> ServerFnError {
    ServerFnError::ServerError(e.to_string())
}

#[cfg(feature = "ssr")]
pub(crate) fn bearer_client(access_token: &str) -> Client {
    Client::builder().token(format!("Bearer {access_token}")).build()
}

#[cfg(feature = "ssr")]
pub(crate) fn db_pool() -> Result<PgPool, ServerFnError> {
    use_context::<PgPool>().ok_or_else(|| {
        ServerFnError::ServerError("missing database pool".to_string())
    })
}

#[cfg(feature = "ssr")]
pub(crate) fn app_state() -> Result<Arc<AppState>, ServerFnError> {
    use_context::<Arc<AppState>>()
        .ok_or_else(|| ServerFnError::ServerError("missing app state".to_string()))
}

#[cfg(feature = "ssr")]
pub(crate) fn discord_client() -> Result<Arc<Client>, ServerFnError> {
    use_context::<Arc<Client>>().ok_or_else(|| {
        ServerFnError::ServerError("missing Discord client".to_string())
    })
}

#[cfg(feature = "ssr")]
pub(crate) async fn current_user_id() -> Result<i64, ServerFnError> {
    let pool = db_pool()?;
    let cookies: Cookies = extract().await.map_err(server_err)?;
    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };
    let row = sqlx::query(
        "SELECT discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(server_err)?;
    let Some(row) = row else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };
    Ok(row.get::<i64, _>("discord_user_id"))
}

#[cfg(feature = "ssr")]
pub(crate) async fn guild_admin_context(
    guild_id_str: &str,
) -> Result<(i64, i64, String), ServerFnError> {
    let Ok(guild_id_i64) = guild_id_str.parse::<i64>() else {
        return Err(ServerFnError::ServerError("invalid guild id".to_string()));
    };
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let pool = db_pool()?;

    let cookies: Cookies = extract().await.map_err(server_err)?;
    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let row = sqlx::query(
        "SELECT discord_access_token, discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(server_err)?;
    let Some(row) = row else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };
    let access_token: String = row.get("discord_access_token");
    let user_id: i64 = row.get("discord_user_id");

    let all_guilds = bearer_client(&access_token)
        .current_user_guilds()
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;
    let has_access = all_guilds.iter().any(|g| {
        g.id.get() == guild_id_u64
            && g.permissions
                .intersects(Permissions::ADMINISTRATOR | Permissions::MANAGE_GUILD)
    });
    if !has_access {
        return Err(ServerFnError::ServerError("forbidden".to_string()));
    }

    Ok((guild_id_i64, user_id, access_token))
}

#[server]
pub async fn check_session() -> Result<bool, ServerFnError> {
    let pool = db_pool()?;

    let cookies: Cookies = extract().await.map_err(server_err)?;

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Ok(false);
    };

    let logged_in = sqlx::query(
        "SELECT 1 FROM web_sessions WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(server_err)?
    .is_some();

    Ok(logged_in)
}
