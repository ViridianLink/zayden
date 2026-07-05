use std::collections::HashMap;

use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::required_option;

use super::MusicCtx;
use crate::error::{MusicError, Result};

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let from: i64 = required_option(&mut options, "from")?;
    let to: i64 = required_option(&mut options, "to")?;
    let from = usize::try_from(from.saturating_sub(1))
        .map_err(|_e| MusicError::QueuePositionOutOfRange(0))?;
    let to = usize::try_from(to.saturating_sub(1))
        .map_err(|_e| MusicError::QueuePositionOutOfRange(0))?;

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::QueueEmpty)?;
    let mut guard = player.lock().await;
    guard.queue.move_song(from, to)?;
    drop(guard);

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().content("Moved the track."),
        )
        .await?;

    Ok(())
}
