use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    leptos_axum::extract,
    palworld::client::PalworldClient,
    sqlx::PgPool,
    std::sync::Arc,
    tower_cookies::Cookies,
    twilight_http::Client,
    twilight_model::guild::Permissions,
    twilight_model::id::Id,
    zayden_app::state::AppState,
};

use crate::dto::SessionUser;

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
pub(crate) fn palworld_client() -> Result<Arc<PalworldClient>, ServerFnError> {
    use_context::<Arc<PalworldClient>>().ok_or_else(|| {
        ServerFnError::ServerError("missing Palworld client".to_string())
    })
}

#[cfg(feature = "ssr")]
pub(crate) async fn current_user_id() -> Result<i64, ServerFnError> {
    let pool = db_pool()?;
    let cookies: Cookies = extract().await.map_err(server_err)?;
    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };
    let row = sqlx::query_scalar!(
        "SELECT discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
        &token,
    )
    .fetch_optional(&pool)
    .await
    .map_err(server_err)?;
    let Some(user_id) = row else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };
    Ok(user_id)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WebRole {
    Admin,
    Operator,
}

impl WebRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Operator => "operator",
        }
    }
}

#[cfg(feature = "ssr")]
pub(crate) async fn has_role(
    pool: &PgPool,
    user_id: i64,
    role: WebRole,
) -> Result<bool, ServerFnError> {
    sqlx::query_scalar!(
        "SELECT 1 FROM web_user_roles WHERE discord_user_id = $1 AND role = $2",
        user_id,
        role.as_str(),
    )
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())
    .map_err(server_err)
}

#[cfg(feature = "ssr")]
pub(crate) async fn require_role(role: WebRole) -> Result<i64, ServerFnError> {
    let user_id = current_user_id().await?;
    let pool = db_pool()?;

    if has_role(&pool, user_id, role).await? {
        Ok(user_id)
    } else {
        Err(ServerFnError::ServerError("forbidden".to_string()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GuildAccess {
    Member,
    Operator,
}

impl GuildAccess {
    #[must_use]
    pub const fn can_write_command_permissions(self) -> bool {
        matches!(self, Self::Member)
    }
}

#[cfg(feature = "ssr")]
pub struct GuildAdminContext {
    pub guild_id: i64,
    pub(crate) access_token: String,
    pub(crate) access: GuildAccess,
}

#[cfg(feature = "ssr")]
async fn bot_is_in_guild(discord: Option<&Client>, guild_id: u64) -> bool {
    let (Some(http), Some(id)) = (discord, Id::new_checked(guild_id)) else {
        return false;
    };

    http.guild(id).await.is_ok()
}

#[cfg(feature = "ssr")]
pub struct SessionIdentity {
    pub user_id: i64,
    pub access_token: String,
}

#[cfg(feature = "ssr")]
pub async fn session_identity(
    pool: &PgPool,
    token: &str,
) -> Result<SessionIdentity, ServerFnError> {
    let row = sqlx::query!(
        "SELECT discord_access_token, discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
        token,
    )
    .fetch_optional(pool)
    .await
    .map_err(server_err)?;

    let Some(row) = row else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    Ok(SessionIdentity {
        user_id: row.discord_user_id,
        access_token: row.discord_access_token,
    })
}

#[cfg(feature = "ssr")]
pub async fn guild_admin_for(
    pool: &PgPool,
    identity: &SessionIdentity,
    guild_id_str: &str,
    discord: Option<&Client>,
) -> Result<GuildAdminContext, ServerFnError> {
    let Ok(guild_id) = guild_id_str.parse::<i64>() else {
        return Err(ServerFnError::ServerError("invalid guild id".to_string()));
    };
    let guild_id_u64 = guild_id.cast_unsigned();

    let all_guilds = bearer_client(&identity.access_token)
        .current_user_guilds()
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;

    let is_member_admin = all_guilds.iter().any(|g| {
        g.id.get() == guild_id_u64
            && g.permissions
                .intersects(Permissions::ADMINISTRATOR | Permissions::MANAGE_GUILD)
    });

    if is_member_admin {
        return Ok(GuildAdminContext {
            guild_id,
            access_token: identity.access_token.clone(),
            access: GuildAccess::Member,
        });
    }

    if !has_role(pool, identity.user_id, WebRole::Operator).await? {
        return Err(ServerFnError::ServerError("forbidden".to_string()));
    }

    if !bot_is_in_guild(discord, guild_id_u64).await {
        return Err(ServerFnError::ServerError(
            "Zayden isn't in that server".to_string(),
        ));
    }

    Ok(GuildAdminContext {
        guild_id,
        access_token: identity.access_token.clone(),
        access: GuildAccess::Operator,
    })
}

#[cfg(feature = "ssr")]
pub(crate) async fn guild_admin_context(
    guild_id_str: &str,
) -> Result<GuildAdminContext, ServerFnError> {
    let pool = db_pool()?;

    let cookies: Cookies = extract().await.map_err(server_err)?;
    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let identity = session_identity(&pool, &token).await?;
    let discord = discord_client().ok();

    guild_admin_for(&pool, &identity, guild_id_str, discord.as_deref()).await
}

#[cfg(feature = "ssr")]
pub(crate) async fn admin_guild_id(guild: &str) -> Result<i64, ServerFnError> {
    guild_admin_context(guild).await.map(|ctx| ctx.guild_id)
}

#[server]
pub async fn check_session() -> Result<bool, ServerFnError> {
    let pool = db_pool()?;

    let cookies: Cookies = extract().await.map_err(server_err)?;

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Ok(false);
    };

    let logged_in = sqlx::query_scalar!(
        "SELECT token FROM web_sessions WHERE token = $1 AND expires_at > now()",
        &token,
    )
    .fetch_optional(&pool)
    .await
    .map_err(server_err)?
    .is_some();

    Ok(logged_in)
}

#[server]
pub async fn current_session_user() -> Result<Option<SessionUser>, ServerFnError> {
    let pool = db_pool()?;

    let cookies: Cookies = extract().await.map_err(server_err)?;

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Ok(None);
    };

    let access_token = sqlx::query_scalar!(
        "SELECT discord_access_token FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
        &token,
    )
    .fetch_optional(&pool)
    .await
    .map_err(server_err)?;

    let Some(access_token) = access_token else {
        return Ok(None);
    };

    let user = match bearer_client(&access_token).current_user().await {
        Ok(response) => response.model().await,
        Err(e) => {
            tracing::warn!(error = ?e, "request to Discord /users/@me failed");
            return Ok(None);
        },
    };

    let user = match user {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(error = ?e, "failed to parse Discord /users/@me response");
            return Ok(None);
        },
    };

    Ok(Some(SessionUser {
        id: user.id.to_string(),
        name: user.global_name.unwrap_or(user.name),
        avatar: user.avatar.map(|hash| hash.to_string()),
    }))
}
