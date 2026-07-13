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
    let query_a: &str =
        required_option(&mut options, "parent_a").map_err(PalworldError::from)?;
    let query_b: &str =
        required_option(&mut options, "parent_b").map_err(PalworldError::from)?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let pals = client.pals().await?;
    let a = find_pal(&pals, query_a).ok_or_else(|| PalworldError::NotFound {
        entity: "pal",
        query: query_a.to_string(),
    })?;
    let b = find_pal(&pals, query_b).ok_or_else(|| PalworldError::NotFound {
        entity: "pal",
        query: query_b.to_string(),
    })?;

    let index = client.breeding_index().await?;

    let child_key =
        index.breed(&a.key, &b.key).map(str::to_string).ok_or_else(|| {
            PalworldError::NotFound {
                entity: "breeding result",
                query: format!("{} × {}", a.name, b.name),
            }
        })?;
    let unique = index.is_unique_child(&child_key);

    let child = find_pal(&pals, &child_key).ok_or_else(|| {
        PalworldError::NotFound { entity: "pal", query: child_key.clone() }
    })?;

    respond(cx, embeds::breeding_component(a, b, child, unique)).await
}
