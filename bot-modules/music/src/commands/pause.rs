use serenity::all::EditInteractionResponse;

use super::MusicCtx;
use crate::error::{MusicError, Result};

pub(super) async fn run(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;
    let guard = player.lock().await;
    let now = guard.current.as_ref().ok_or(MusicError::NothingPlaying)?;
    now.handle.pause().map_err(|e| MusicError::Songbird(e.to_string()))?;
    drop(guard);

    ctx.interaction
        .edit_response(ctx.http, EditInteractionResponse::new().content("Paused."))
        .await?;

    Ok(())
}
