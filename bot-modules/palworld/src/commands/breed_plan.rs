use std::collections::HashMap;

use serenity::all::ResolvedValue;
use sqlx::PgPool;
use zayden_core::{InvocationCtx, optional_option, required_option};

use super::{find_pal, resolve_player, respond};
use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};
use crate::model::{OwnedPal, Pal};
use crate::{difficulty, embeds, save};

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    client: &PalworldClient,
    pool: &PgPool,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let query: &str =
        required_option(&mut options, "target").map_err(PalworldError::from)?;
    let player: Option<&str> = optional_option(&mut options, "player");

    cx.interaction.defer(&cx.ctx.http).await?;

    let roster = resolve_player(cx, client, pool, player).await?;
    let pals = client.pals().await?;

    let target = find_pal(&pals, query).ok_or_else(|| PalworldError::NotFound {
        entity: "pal",
        query: query.to_string(),
    })?;

    let owned: Vec<OwnedPal> = roster
        .pals
        .iter()
        .filter_map(|p| {
            save::palmap::resolve_species(&p.species, &pals).map(|species| {
                OwnedPal { species, gender: p.gender, nickname: p.nickname.clone() }
            })
        })
        .collect();

    let base_cost: HashMap<String, i64> = pals
        .iter()
        .map(|p| (p.key.clone(), difficulty::pal_difficulty(p)))
        .collect();

    let index = client.breeding_index().await?;
    let Some(plan) = index.plan(&owned, &target.key, &base_cost) else {
        return respond(cx, embeds::breed_plan_unreachable_component(target)).await;
    };

    let lookup: HashMap<&str, &Pal> =
        pals.iter().map(|p| (p.key.as_str(), p)).collect();
    let display = |key: &str| -> String {
        lookup.get(key).map_or_else(|| key.to_string(), |p| p.name.clone())
    };

    let steps: Vec<String> = plan
        .steps
        .iter()
        .map(|step| {
            let mark = if step.ready { "✅" } else { "⏳" };
            format!(
                "{mark} **{}** × **{}** → **{}**",
                display(&step.pair.a),
                display(&step.pair.b),
                display(&step.child),
            )
        })
        .collect();

    let leaves: Vec<String> =
        plan.leaves_to_obtain.iter().map(|key| display(key)).collect();

    respond(
        cx,
        embeds::breed_plan_component(target, &steps, &leaves, plan.total_cost),
    )
    .await
}
