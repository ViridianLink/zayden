use std::collections::HashMap;

use serenity::all::ResolvedValue;
use zayden_core::{InvocationCtx, required_option};

use super::{find_pal, respond};
use crate::client::PalworldClient;
use crate::embeds;
use crate::error::{PalworldError, Result};

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    client: &PalworldClient,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let query: &str =
        required_option(&mut options, "name").map_err(PalworldError::from)?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let pals = client.pals().await?;
    let key = find_pal(&pals, query)
        .ok_or_else(|| PalworldError::NotFound {
            entity: "pal",
            query: query.to_string(),
        })?
        .key
        .clone();

    let pal = client.pal(&key).await?;
    respond(cx, embeds::pal_component(&pal)).await
}
