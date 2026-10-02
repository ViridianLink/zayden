use std::fmt;
use std::sync::Arc;

use sqlx::PgPool;
use topcoat::context::Cx;
use topcoat::cookie::Cookies;
use twilight_http::Client;
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;

use super::context::{cookie_jar, db_pool, try_web_state};
use super::cookie::SESSION_COOKIE;
use super::discord::bearer_client;
use super::error::AuthError;
use super::roles::{WebRole, has_role};
use super::session::session_for_token;
use crate::state::{SessionIdentity, UserGuildsCache};

/// How the signed-in user reached a guild's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildAccess {
    /// The user manages the guild in Discord.
    Member,
    /// A dashboard operator viewing a guild the bot is in.
    Operator,
}

impl GuildAccess {
    /// Discord accepts command-permission writes only from a bearer token of
    /// a member who manages the guild.
    #[must_use]
    pub const fn can_write_command_permissions(self) -> bool {
        matches!(self, Self::Member)
    }
}

/// Proof that the signed-in user may manage a guild. `access_token` is the
/// user's Discord bearer token, so `Debug` leaves it out.
#[derive(Clone, PartialEq, Eq)]
pub struct GuildAdminContext {
    pub guild_id: i64,
    pub access_token: String,
    pub access: GuildAccess,
}

impl fmt::Debug for GuildAdminContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GuildAdminContext")
            .field("guild_id", &self.guild_id)
            .field("access", &self.access)
            .finish_non_exhaustive()
    }
}

/// Administrator or Manage Server, the rights that let a member manage a
/// guild here.
#[must_use]
pub fn manages_guild(guild: &CurrentUserGuild) -> bool {
    guild
        .permissions
        .intersects(Permissions::ADMINISTRATOR | Permissions::MANAGE_GUILD)
}

#[must_use]
pub fn find_user_guild(
    guilds: &[CurrentUserGuild],
    guild_id: u64,
) -> Option<&CurrentUserGuild> {
    guilds.iter().find(|g| g.id.get() == guild_id)
}

/// The user's guilds from Discord's `GET /users/@me/guilds`, through the
/// cache.
///
/// This is the only call of that endpoint: the server switcher and every
/// per-guild authorization read this one payload. The cache TTL bounds how
/// long a revoked guild permission stays honoured.
pub async fn lookup_user_guilds(
    cache: Option<&UserGuildsCache>,
    identity: &SessionIdentity,
) -> Result<Arc<[CurrentUserGuild]>, AuthError> {
    if let Some(cache) = cache
        && let Some(guilds) = cache.get(&identity.user_id).await
    {
        return Ok(guilds);
    }

    let guilds: Arc<[CurrentUserGuild]> = bearer_client(&identity.access_token)
        .current_user_guilds()
        .await?
        .model()
        .await?
        .into();

    if let Some(cache) = cache {
        cache.insert(identity.user_id, Arc::clone(&guilds)).await;
    }

    Ok(guilds)
}

/// [`lookup_user_guilds`] through the app's cache.
pub async fn user_guilds(
    cx: &Cx,
    identity: &SessionIdentity,
) -> Result<Arc<[CurrentUserGuild]>, AuthError> {
    let cache = try_web_state(cx).map(|state| &state.discord.user_guilds);
    lookup_user_guilds(cache, identity).await
}

async fn bot_is_in_guild(discord: Option<&Client>, guild_id: u64) -> bool {
    let (Some(http), Some(id)) = (discord, Id::new_checked(guild_id)) else {
        return false;
    };

    http.guild(id).await.is_ok()
}

/// Decides whether `identity` may manage the guild `guild_id_str`: as a
/// member with Administrator or Manage Server, or as an operator when the bot
/// is in the guild.
pub async fn guild_admin_for(
    pool: &PgPool,
    identity: &SessionIdentity,
    guild_id_str: &str,
    discord: Option<&Client>,
    guilds_cache: Option<&UserGuildsCache>,
) -> Result<GuildAdminContext, AuthError> {
    let Ok(guild_id) = guild_id_str.parse::<i64>() else {
        return Err(AuthError::InvalidGuildId);
    };
    let guild_id_u64 = guild_id.cast_unsigned();

    let all_guilds = lookup_user_guilds(guilds_cache, identity).await?;

    let is_member_admin =
        find_user_guild(&all_guilds, guild_id_u64).is_some_and(manages_guild);

    if is_member_admin {
        return Ok(GuildAdminContext {
            guild_id,
            access_token: identity.access_token.clone(),
            access: GuildAccess::Member,
        });
    }

    if !has_role(pool, identity.user_id, WebRole::Operator).await? {
        return Err(AuthError::Forbidden);
    }

    if !bot_is_in_guild(discord, guild_id_u64).await {
        return Err(AuthError::BotNotInGuild);
    }

    Ok(GuildAdminContext {
        guild_id,
        access_token: identity.access_token.clone(),
        access: GuildAccess::Operator,
    })
}

/// [`guild_admin_for`] the request's session.
pub async fn guild_admin_context(
    cx: &Cx,
    guild_id_str: &str,
) -> Result<GuildAdminContext, AuthError> {
    let pool = db_pool(cx)?;

    let Some(token) = cookie_jar(cx)?.get(SESSION_COOKIE) else {
        return Err(AuthError::Unauthenticated);
    };

    let identity = session_for_token(cx, token.value())
        .await?
        .ok_or(AuthError::Unauthenticated)?;
    let state = try_web_state(cx);

    guild_admin_for(
        pool,
        &identity,
        guild_id_str,
        state.map(|state| state.discord.http.as_ref()),
        state.map(|state| &state.discord.user_guilds),
    )
    .await
}

pub async fn admin_guild_id(cx: &Cx, guild: &str) -> Result<i64, AuthError> {
    guild_admin_context(cx, guild).await.map(|ctx| ctx.guild_id)
}
