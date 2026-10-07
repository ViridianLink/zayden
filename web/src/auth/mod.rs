mod context;
mod cookie;
mod discord;
mod dto;
mod error;
mod guild;
mod login;
mod middleware;
mod ownership;
mod roles;
mod session;
pub mod supersede;

pub(crate) use context::cookie_jar;
pub use context::{app_state, db_pool, discord_client, web_state};
pub use cookie::{
    OAUTH_STATE_COOKIE,
    SESSION_COOKIE,
    SESSION_TTL_HOURS,
    build as build_cookie,
};
pub use discord::{
    ANNOUNCE_PERMISSIONS,
    bearer_client,
    channel_permissions,
    ensure_bot_can_post,
    ensure_guild_channel,
    list_guild_channels,
    list_guild_roles,
};
pub(crate) use discord::{fetch_guild_channels, fetch_guild_roles};
pub use dto::{ChannelInfo, ForumTagInfo, RoleInfo, SessionUser};
pub use error::{AuthError, FORBIDDEN, ForeignIdError, UNAUTHENTICATED};
pub use guild::{
    GuildAccess,
    GuildAdminContext,
    admin_guild_id,
    find_user_guild,
    guild_admin_context,
    guild_admin_for,
    lookup_user_guilds,
    manages_guild,
    user_guilds,
};
pub use login::start_session;
pub use middleware::require_auth;
pub use ownership::{GuildIds, first_foreign};
pub use roles::{WebRole, has_role, require_role};
pub use session::{
    LOGIN_PATH,
    check_session,
    current_session_identity,
    current_session_user,
    current_user_id,
    lookup_session,
    lookup_session_user,
    require_user,
};
use topcoat::router::RouterBuilder;

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.route(login::login_handler)
        .route(login::discord_auth_callback_handler)
        .route(login::logout_handler)
}
