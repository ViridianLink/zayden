use std::sync::Arc;

use serenity::all::EditInteractionResponse;
use tracing::warn;
use zayden_core::InvocationCtx;

use crate::error::Result;
use crate::identity::{JellyfinLinkRow, quick_connect};
use crate::runtime::JellyfinRuntime;

pub async fn run(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
) -> Result<()> {
    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    let user_id = cx.interaction.user.id;
    let removed = JellyfinLinkRow::delete(&cx.app.db, user_id).await?;

    // Clear any session a previous link left behind, so re-linking starts clean.
    let device_id = quick_connect::device_id(user_id);
    if let Err(e) = runtime.jellyfin.revoke_device(&device_id).await {
        warn!(error = ?e, %user_id, "could not revoke the device on unlink");
    }

    let content = if removed {
        "Unlinked. I can no longer see your watch history or request on your \
         behalf. Your Jellyfin account itself is untouched."
    } else {
        "You were not linked to a Jellyfin account."
    };

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().content(content))
        .await?;

    Ok(())
}
