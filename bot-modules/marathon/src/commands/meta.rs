use serenity::all::{EditInteractionResponse, MessageFlags};
use zayden_core::InvocationCtx;

use crate::client::MarathonClient;
use crate::embeds;
use crate::error::Result;

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    client: &MarathonClient,
) -> Result<()> {
    cx.interaction.defer(&cx.ctx.http).await?;

    let entries = client.meta().await?;
    let component = embeds::meta_component(&entries);

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .flags(MessageFlags::IS_COMPONENTS_V2)
                .components(vec![component]),
        )
        .await?;

    Ok(())
}
