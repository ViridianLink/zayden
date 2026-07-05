use serenity::all::EditInteractionResponse;

use super::MusicCtx;
use crate::error::{MusicError, Result};
use crate::voice;

pub(super) async fn run(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;

    let (old_handle, next, generation) = {
        let mut guard = player.lock().await;
        guard.current.as_ref().ok_or(MusicError::NothingPlaying)?;
        let old_handle = guard.current.as_ref().map(|now| now.handle.clone());
        let next = guard.advance_queue();
        (old_handle, next, guard.generation)
    };

    let started_next = next.is_some();
    voice::stop_current_and_start(
        &ctx.songbird,
        &ctx.music,
        &ctx.resolver,
        ctx.guild_id,
        old_handle,
        next,
        generation,
    )
    .await?;

    let message = if started_next {
        "Force-skipped. Playing the next track."
    } else {
        "Force-skipped. The queue is now empty."
    };

    ctx.interaction
        .edit_response(ctx.http, EditInteractionResponse::new().content(message))
        .await?;

    Ok(())
}
