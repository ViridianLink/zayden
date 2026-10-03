use topcoat::context::Cx;

use crate::auth::{
    GuildAdminContext,
    current_session_identity,
    guild_admin_for,
    web_state,
};

/// A signed-in user who administers the guild they asked about.
pub(super) struct GuildAdmin {
    pub(super) context: GuildAdminContext,
    pub(super) user_id: i64,
}

/// The caller's guild-admin standing for `guild`, or `None` for any refusal
/// or failure.
pub(super) async fn guild_admin(cx: &Cx, guild: &str) -> Option<GuildAdmin> {
    let state = web_state(cx).ok()?;
    let identity = current_session_identity(cx).await.ok()??;
    let user_id = identity.user_id;

    let context = guild_admin_for(
        &state.app.db,
        &identity,
        guild,
        Some(&state.discord.http),
        Some(&state.discord.user_guilds),
    )
    .await
    .ok()?;

    Some(GuildAdmin { context, user_id })
}
