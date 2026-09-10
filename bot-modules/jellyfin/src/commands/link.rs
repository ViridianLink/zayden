use std::sync::Arc;

use serenity::all::{CreateComponent, EditInteractionResponse};
use zayden_core::InvocationCtx;

use crate::error::Result;
use crate::identity::quick_connect;
use crate::runtime::JellyfinRuntime;

pub async fn run(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
) -> Result<()> {
    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    let user_id = cx.interaction.user.id;
    let pending = quick_connect::begin(runtime, &cx.app.db, user_id).await?;

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .content(quick_connect::prompt(
                    &pending,
                    &runtime.jellyfin.quick_connect_url(),
                ))
                .components(vec![CreateComponent::ActionRow(
                    quick_connect::cancel_button(user_id),
                )]),
        )
        .await?;

    // The poller owns the rest of the flow and edits this same response when it
    // resolves, so the command returns immediately.
    quick_connect::spawn_poller(
        Arc::clone(&cx.ctx.http),
        cx.interaction.token.to_string(),
        Arc::clone(runtime),
        cx.app.db.clone(),
        user_id,
        cx.interaction.user.name.to_string(),
    );

    Ok(())
}
