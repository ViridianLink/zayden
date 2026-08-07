use serenity::all::EditInteractionResponse;

use super::MusicCtx;
use crate::error::{MusicError, Result};
use crate::voice;

pub(super) async fn run(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    if voice::get_call(&ctx.songbird, ctx.guild_id).is_none() {
        return Err(MusicError::NotConnected);
    }

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    voice::end_session(&ctx.songbird, &ctx.music, ctx.guild_id).await?;

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().content("Disconnected."),
        )
        .await?;

    Ok(())
}
