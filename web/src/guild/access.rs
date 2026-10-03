use std::sync::Arc;

use topcoat::context::Cx;
use zayden_app::state::AppState as ZaydenAppState;

use super::error::GuildError;
use crate::auth::{admin_guild_id, app_state};

/// The guild id `guild` names, once the signed-in user may manage it, with
/// the app state settings are read and written through.
pub async fn admin_app<'a>(
    cx: &'a Cx,
    guild: &str,
) -> Result<(i64, &'a Arc<ZaydenAppState>), GuildError> {
    let guild_id = admin_guild_id(cx, guild).await?;
    Ok((guild_id, app_state(cx)?))
}
