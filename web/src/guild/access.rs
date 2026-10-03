use std::sync::Arc;

use topcoat::context::Cx;
use zayden_app::state::AppState as ZaydenAppState;

use super::error::GuildError;
use crate::auth::{admin_guild_id, app_state};

pub async fn admin_app<'a>(
    cx: &'a Cx,
    guild: &str,
) -> Result<(i64, &'a Arc<ZaydenAppState>), GuildError> {
    let guild_id = admin_guild_id(cx, guild).await?;
    Ok((guild_id, app_state(cx)?))
}
