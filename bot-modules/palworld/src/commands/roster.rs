use std::collections::HashMap;

use serenity::all::ResolvedValue;
use sqlx::PgPool;
use zayden_core::{InvocationCtx, optional_option};

use super::{resolve_player, respond};
use crate::client::PalworldClient;
use crate::error::Result;
use crate::model::Gender;
use crate::{embeds, save};

const MAX_SPECIES: usize = 40;

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    client: &PalworldClient,
    pool: &PgPool,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let player: Option<&str> = optional_option(&mut options, "player");

    cx.interaction.defer(&cx.ctx.http).await?;

    let roster = resolve_player(cx, client, pool, player).await?;
    let pals = client.pals().await?;

    let mut counts: HashMap<String, [usize; 3]> = HashMap::new();
    let mut total = 0usize;
    for pal in &roster.pals {
        let Some(key) = save::palmap::resolve_species(&pal.species, &pals) else {
            continue;
        };
        let name =
            pals.iter().find(|p| p.key == key).map_or(key, |p| p.name.clone());
        let entry = counts.entry(name).or_default();
        match pal.gender {
            Gender::Male => entry[0] += 1,
            Gender::Female => entry[1] += 1,
            Gender::Unknown => entry[2] += 1,
        }
        total += 1;
    }

    let total_species = counts.len();
    let mut ordered: Vec<(String, [usize; 3])> = counts.into_iter().collect();
    ordered.sort_by(|a, b| {
        let (sa, sb) = (a.1.iter().sum::<usize>(), b.1.iter().sum::<usize>());
        sb.cmp(&sa).then_with(|| a.0.cmp(&b.0))
    });

    let lines: Vec<String> = ordered
        .iter()
        .take(MAX_SPECIES)
        .map(|(name, [m, f, u])| {
            let mut breakdown = Vec::new();
            if *m > 0 {
                breakdown.push(format!("♂{m}"));
            }
            if *f > 0 {
                breakdown.push(format!("♀{f}"));
            }
            if *u > 0 {
                breakdown.push(format!("·{u}"));
            }
            format!("- **{name}** ×{} ({})", m + f + u, breakdown.join(" "))
        })
        .collect();

    let hidden = total_species.saturating_sub(lines.len());
    respond(cx, embeds::roster_component(&roster.name, total, &lines, hidden)).await
}
