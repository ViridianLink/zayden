use std::collections::HashMap;

use serenity::all::ResolvedValue;
use zayden_core::{InvocationCtx, required_option};

use super::{find_pal, respond};
use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};
use crate::model::Pal;
use crate::{difficulty, embeds};

const MAX_PAIRS: usize = 25;

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    client: &PalworldClient,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let query: &str =
        required_option(&mut options, "target").map_err(PalworldError::from)?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let pals = client.pals().await?;
    let target = find_pal(&pals, query).ok_or_else(|| PalworldError::NotFound {
        entity: "pal",
        query: query.to_string(),
    })?;

    let index = client.breeding_index().await?;
    let mut all = index.breed_for(&target.key);
    let total = all.len();

    let lookup: HashMap<&str, &Pal> =
        pals.iter().map(|p| (p.key.as_str(), p)).collect();
    all.sort_by_cached_key(|pair| {
        match (lookup.get(pair.a.as_str()), lookup.get(pair.b.as_str())) {
            (Some(a), Some(b)) => difficulty::pair_difficulty(a, b),
            _ => (i64::MAX, i64::MAX),
        }
    });

    let display = |key: &str| -> String {
        find_pal(&pals, key).map_or_else(|| key.to_string(), |p| p.name.clone())
    };
    let pairs: Vec<(String, String)> = all
        .iter()
        .take(MAX_PAIRS)
        .map(|pair| (display(&pair.a), display(&pair.b)))
        .collect();

    respond(cx, embeds::breed_for_component(target, &pairs, total)).await
}
