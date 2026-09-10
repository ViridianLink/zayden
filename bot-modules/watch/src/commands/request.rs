use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::identity::{JellyfinLinkRow, seer_user};
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, optional_option, required_option};

use crate::discovery::{Resolved, resolve};
use crate::embeds;
use crate::error::Result;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let title: &str = required_option(&mut options, "title")?;
    let seasons: Option<&str> = optional_option(&mut options, "seasons");

    cx.interaction.defer(&cx.ctx.http).await?;

    let resolved = resolve(runtime, &cx.app.db, title).await?;

    // Already here: hand over the link and file nothing.
    if let Resolved::OnServer { .. } = &resolved {
        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new()
                    .embed(embeds::media::resolved(&runtime.jellyfin, &resolved)),
            )
            .await?;
        return Ok(());
    }

    let tmdb_id = resolved
        .tmdb_id()
        .ok_or_else(|| JellyfinError::NoSuchTitle(title.to_owned()))?;

    let link = JellyfinLinkRow::require(&cx.app.db, cx.interaction.user.id).await?;
    let seer_id = seer_user::resolve(runtime, &cx.app.db, &link).await?;

    let request = runtime
        .seer
        .create_request(resolved.kind(), tmdb_id, season_payload(seasons), seer_id)
        .await
        .map_err(JellyfinError::from)?;

    // Report what Jellyseerr decided. Approval follows the requesting user's own
    // permissions there, so predicting it in the bot would be a lie the first
    // time those permissions change.
    let verdict = if request.is_approved() {
        "Approved — it will start downloading shortly."
    } else if request.is_pending() {
        "Filed, and waiting on manual approval."
    } else {
        "Filed."
    };

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().embed(
                embeds::media::resolved(&runtime.jellyfin, &resolved)
                    .field("Request", verdict, false),
            ),
        )
        .await?;

    Ok(())
}

fn season_payload(seasons: Option<&str>) -> Option<serde_json::Value> {
    match seasons {
        Some("first") => Some(serde_json::json!([1])),
        Some("latest") => Some(serde_json::json!("latest")),
        Some("all") | None => Some(serde_json::json!("all")),
        Some(_) => None,
    }
}
