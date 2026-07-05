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

    let pos: i64 = required_option(&mut options, "position")?;
    let pos = usize::try_from(pos.saturating_sub(1))
        .map_err(|_e| MusicError::QueuePositionOutOfRange(0))?;

    let settings = ctx.settings().await?;
    let privileged = ctx.is_privileged(&settings);
    let user_id = ctx.interaction.user.id;

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::QueueEmpty)?;
    let mut guard = player.lock().await;

    let is_own_track = guard
        .queue
        .get(pos)
        .is_some_and(|track| track.requested_by.user_id == user_id);
    if !privileged && !is_own_track {
        return Err(MusicError::NotPrivileged);
    }

    let removed = guard.queue.remove(pos)?;
    drop(guard);

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new()
                .content(format!("Removed **{}** from the queue.", removed.title)),
        )
        .await?;

    Ok(())
}
