use axum::Extension;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use tracing::warn;

use crate::WebState;
use crate::middleware::auth::AuthToken;

const DISCORD_API: &str = "https://discord.com/api/v10";
const MANAGE_GUILD: u64 = 0x0000_0000_0000_0020;
const ADMINISTRATOR: u64 = 0x0000_0000_0000_0008;

#[derive(Deserialize)]
struct PartialGuild {
    id: String,
    permissions: String,
}

pub(crate) async fn require_guild_permission(
    State(state): State<WebState>,
    Extension(AuthToken(token)): Extension<AuthToken>,
    req: Request,
    next: Next,
) -> Response {
    let Some(guild_id) = guild_id_from_path(req.uri().path()) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let Some(guilds) = fetch_user_guilds(&state, &token).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let can_manage = guilds.iter().any(|g| {
        g.id.parse::<u64>().ok() == Some(guild_id)
            && g.permissions
                .parse::<u64>()
                .is_ok_and(|p| p & ADMINISTRATOR != 0 || p & MANAGE_GUILD != 0)
    });

    if !can_manage {
        return StatusCode::FORBIDDEN.into_response();
    }

    next.run(req).await
}

fn guild_id_from_path(path: &str) -> Option<u64> {
    // All guild-scoped routes have the form /guild/{id}[/...].
    path.split('/').nth(2)?.parse().ok()
}

async fn fetch_user_guilds(
    state: &WebState,
    token: &str,
) -> Option<Vec<PartialGuild>> {
    match state
        .app
        .http
        .get(format!("{DISCORD_API}/users/@me/guilds"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp.json().await.ok(),
        Ok(resp) => {
            warn!(status = %resp.status(), "Discord /users/@me/guilds returned error");
            None
        },
        Err(e) => {
            warn!(?e, "Discord /users/@me/guilds request failed");
            None
        },
    }
}
