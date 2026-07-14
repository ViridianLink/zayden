use std::collections::{HashMap, HashSet};

use serenity::all::ResolvedValue;
use sqlx::PgPool;
use zayden_core::{InvocationCtx, optional_option, required_option};

use super::{find_pal, resolve_player, respond};
use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};
use crate::model::{Gender, OwnedPal, Pal};
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

    let mut owned_gender: HashMap<&str, (bool, bool)> = HashMap::new();
    for p in &owned {
        let entry = owned_gender.entry(p.species.as_str()).or_insert((false, false));
        match p.gender {
            Gender::Male => entry.0 = true,
            Gender::Female => entry.1 = true,
            Gender::Unknown => {},
        }
    }

    let leaves_set: HashSet<&str> =
        plan.leaves_to_obtain.iter().map(String::as_str).collect();

    let parent = |key: &str| -> String {
        if leaves_set.contains(key) {
            format!("**{}** 📥", display(key))
        } else {
            format!("**{}**", display(key))
        }
    };

    let mut to_obtain: Vec<String> =
        plan.leaves_to_obtain.iter().map(|key| display(key)).collect();
    let mut seen_gender: HashSet<(String, String)> = HashSet::new();

    let steps: Vec<String> = plan
        .steps
        .iter()
        .map(|step| {
            let a = step.pair.a.as_str();
            let b = step.pair.b.as_str();
            let mark = if step.ready { "✅" } else { "⏳" };
            let mut line = format!(
                "{mark} {} × {} → **{}**",
                parent(a),
                parent(b),
                display(&step.child),
            );

            if !step.ready
                && let Some(need) = gender_gap(a, b, &owned_gender, &display)
            {
                if seen_gender.insert((a.to_string(), b.to_string())) {
                    to_obtain.push(need);
                }
                line.push_str(" — wrong gender");
            }

            line
        })
        .collect();

    respond(
        cx,
        embeds::breed_plan_component(target, &steps, &to_obtain, plan.total_cost),
    )
    .await
}

fn gender_gap(
    a: &str,
    b: &str,
    owned_gender: &HashMap<&str, (bool, bool)>,
    display: &impl Fn(&str) -> String,
) -> Option<String> {
    let ga = owned_gender.get(a).copied()?;
    if a == b {
        let missing = match ga {
            (true, false) => "female",
            (false, true) => "male",
            _ => return None,
        };
        return Some(format!("another **{}** ({missing})", display(a)));
    }

    let gb = owned_gender.get(b).copied()?;
    if (ga.0 && gb.1) || (ga.1 && gb.0) {
        return None;
    }
    Some(format!("an opposite-gender **{}** or **{}**", display(a), display(b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned<'a>(
        entries: &[(&'a str, (bool, bool))],
    ) -> HashMap<&'a str, (bool, bool)> {
        entries.iter().copied().collect()
    }

    fn id(key: &str) -> String {
        key.to_string()
    }

    #[test]
    fn same_species_single_gender_needs_opposite() {
        let g = owned(&[("A", (true, false))]);
        assert_eq!(
            gender_gap("A", "A", &g, &id),
            Some("another **A** (female)".to_string())
        );

        let g = owned(&[("A", (false, true))]);
        assert_eq!(
            gender_gap("A", "A", &g, &id),
            Some("another **A** (male)".to_string())
        );
    }

    #[test]
    fn same_species_both_genders_has_no_gap() {
        let g = owned(&[("A", (true, true))]);
        assert_eq!(gender_gap("A", "A", &g, &id), None);
    }

    #[test]
    fn different_species_same_gender_needs_opposite() {
        let g = owned(&[("A", (true, false)), ("B", (true, false))]);
        assert_eq!(
            gender_gap("A", "B", &g, &id),
            Some("an opposite-gender **A** or **B**".to_string())
        );
    }

    #[test]
    fn different_species_compatible_has_no_gap() {
        let g = owned(&[("A", (true, false)), ("B", (false, true))]);
        assert_eq!(gender_gap("A", "B", &g, &id), None);
    }

    #[test]
    fn unowned_parent_yields_no_gap() {
        // A parent that will be caught or bred is not an owned-gender problem.
        let g = owned(&[("A", (true, false))]);
        assert_eq!(gender_gap("A", "B", &g, &id), None);
        assert_eq!(gender_gap("X", "Y", &g, &id), None);
    }
}
