use std::collections::HashMap;

use serenity::all::ResolvedValue;
use zayden_core::{InvocationCtx, required_option};

use super::respond;
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

    let items = client.items().await?;
    let key = find_item_key(&items, query).ok_or_else(|| {
        PalworldError::NotFound { entity: "item", query: query.to_string() }
    })?;

    let item = client.item(&key).await?;
    respond(cx, embeds::item_component(&item)).await
}

fn find_item_key(items: &[crate::model::Item], query: &str) -> Option<String> {
    if let Some(item) = items.iter().find(|i| i.key == query) {
        return Some(item.key.clone());
    }
    if let Some(item) = items.iter().find(|i| i.name.eq_ignore_ascii_case(query)) {
        return Some(item.key.clone());
    }
    let query_lower = query.to_lowercase();
    items
        .iter()
        .find(|i| i.name.to_lowercase().contains(&query_lower))
        .map(|i| i.key.clone())
}
