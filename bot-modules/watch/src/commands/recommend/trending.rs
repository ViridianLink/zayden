use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::EditInteractionResponse;
use zayden_core::InvocationCtx;

use crate::embeds;
use crate::error::Result;

pub async fn run(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
) -> Result<()> {
    cx.interaction.defer(&cx.ctx.http).await?;

    let results = runtime.seer.trending().await.map_err(JellyfinError::from)?;

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().embed(embeds::media::list(
                "Trending now",
                &results,
                Some("Anything not marked as on the server can be requested with `/watch request`."),
            )),
        )
        .await?;

    Ok(())
}
