use std::collections::HashMap;

use serenity::all::{EditInteractionResponse, MessageFlags, ResolvedValue};
use zayden_core::{InvocationCtx, required_option};

use super::find_entity;
use crate::client::MarathonClient;
use crate::embeds;
use crate::error::{MarathonError, Result};

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    client: &MarathonClient,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let query: &str =
        required_option(&mut options, "name").map_err(MarathonError::from)?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let builds = client.builds().await?;
    let build =
        find_entity(&builds, query, |b| b.slug.as_str(), |b| b.name.as_str())
            .ok_or_else(|| MarathonError::NotFound {
                entity: "build",
                query: query.to_string(),
            })?;

    let component = embeds::build_component(build);

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
