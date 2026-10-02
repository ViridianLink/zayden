//! Sessions, roles, guild access and the Discord sign-in routes.
//!
//! Pages and routes authorize themselves by calling these helpers with their
//! `cx`. Every helper returns `Result<_, AuthError>`, and each error's
//! `Display` is the exact message the dashboard shows: render it inline with
//! [`server_error_text`](crate::util::server_error_text), and check
//! [`AuthError::is_denied`] to tell a refusal (`unauthenticated`,
//! `forbidden`) from a failure.
//!
//! - [`current_session_identity`], [`check_session`] and [`current_session_user`]
//!   read the `session` cookie without requiring it.
//! - [`require_user`] is [`AuthError::Unauthenticated`] without a live session.
//!   Members-only pages call [`AuthError::redirect_unauthenticated`] on its error:
//!   `?` on the result answers a signed-out visitor with a 303 to `/login`, and any
//!   other error comes back to be rendered inline.
//! - [`current_user_id`], [`require_role`] and [`guild_admin_context`] fail with
//!   `unauthenticated`, `forbidden`, `invalid guild id` or `Zayden isn't in that
//!   server`; guild pages send `unauthenticated` through
//!   [`AuthError::redirect_unauthenticated`] too.
//! - [`list_guild_channels`], [`list_guild_roles`], [`GuildIds`] and
//!   [`ensure_bot_can_post`] read and validate Discord guild data.
//! - [`supersede::claim`] serializes writes to one guild module.
//! - [`require_auth`] guards a route that is not a page with an empty 401.
//!
//! The router must install `.cookies()`. Helpers run inside that `/` cookie
//! layer, so a pathless layer (one registered without a path) cannot call
//! them. Without the layer they fail with `missing cookie jar`.

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
    require_user,
};
use topcoat::router::RouterBuilder;

/// Registers the Discord sign-in routes: `GET /auth/discord`,
/// `GET /auth/callback` and `GET /logout`.
#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.route(login::login_handler)
        .route(login::discord_auth_callback_handler)
        .route(login::logout_handler)
}
