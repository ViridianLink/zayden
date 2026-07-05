use serenity::all::EditInteractionResponse;

use super::MusicCtx;
use crate::error::{MusicError, Result};

pub(super) async fn run(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let bot_channel = ctx
        .music
        .occupancy()
        .channel_of(ctx.guild_id, ctx.bot_id)
        .ok_or(MusicError::NotConnected)?;
    let voice_members =
        ctx.music.occupancy().members_in_channel(ctx.guild_id, bot_channel);

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::QueueEmpty)?;
    let mut guard = player.lock().await;
    let removed = guard.queue.cleanup(&voice_members);
    drop(guard);

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().content(format!(
                "Removed {removed} track(s) requested by members who left voice."
            )),
        )
        .await?;

    Ok(())
}
