use std::collections::HashMap;

use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::required_option;

use super::MusicCtx;
use crate::error::{MusicError, Result};
use crate::voice;

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let pos: i64 = required_option(&mut options, "position")?;
    let pos = usize::try_from(pos.saturating_sub(1))
        .map_err(|_e| MusicError::QueuePositionOutOfRange(0))?;

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;

    let (old_handle, next, generation) = {
        let mut guard = player.lock().await;
        let next = guard.queue.skip_to(pos)?;
        let old_handle = guard.current.as_ref().map(|now| now.handle.clone());
        guard.advance();
        (old_handle, next, guard.generation)
    };

    voice::stop_current_and_start(
        &ctx.songbird,
        &ctx.music,
        &ctx.resolver,
        ctx.guild_id,
        old_handle,
        Some(next),
        generation,
    )
    .await?;

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().content("Skipped to that track."),
        )
        .await?;

    Ok(())
}
