use serenity::all::EditInteractionResponse;

use super::MusicCtx;
use crate::error::Result;
use crate::voice;

pub(super) async fn run(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    let request = ctx.session_request(&settings);

    let (channel_id, _call) =
        voice::ensure_session(&ctx.songbird, &ctx.music, request).await?;

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new()
                .content(format!("Joined <#{channel_id}>.")),
        )
        .await?;

    Ok(())
}
