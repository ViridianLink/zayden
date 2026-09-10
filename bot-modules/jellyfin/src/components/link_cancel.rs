use std::sync::Arc;

use serenity::all::{CreateInteractionResponse, EditInteractionResponse};
use zayden_core::ComponentCtx;

use crate::error::Result;
use crate::identity::quick_connect;
use crate::runtime::JellyfinRuntime;

pub async fn run(
    cx: &ComponentCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    suffix: &str,
) -> Result<()> {
    let owner = suffix.parse::<u64>().ok();
    if owner != Some(cx.interaction.user.id.get()) {
        cx.interaction
            .create_response(&cx.ctx.http, CreateInteractionResponse::Acknowledge)
            .await?;
        return Ok(());
    }

    quick_connect::cancel(runtime, cx.interaction.user.id).await;

    cx.interaction.defer(&cx.ctx.http).await?;
    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .content("Link cancelled.")
                .components(vec![]),
        )
        .await?;

    Ok(())
}
