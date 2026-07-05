use serenity::all::EditInteractionResponse;

use super::MusicCtx;
use crate::error::{MusicError, Result};

pub(super) async fn run(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::QueueEmpty)?;
    let mut guard = player.lock().await;
    guard.queue.shuffle();
    drop(guard);

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().content("Shuffled the queue."),
        )
        .await?;

    Ok(())
}
