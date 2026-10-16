use std::collections::HashMap;

use deadlock_api::Priority;
use serenity::all::{EditInteractionResponse, MessageFlags, ResolvedValue};
use zayden_core::{InvocationCtx, optional_option, required_option};

use super::{resolve_hero, resolve_rank};
use crate::analysis::lane_summary;
use crate::client::DeadlockApi;
use crate::embeds::{self, Freshness};
use crate::error::{DeadlockError, Result};

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    api: &DeadlockApi,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let hero_query: &str =
        required_option(&mut options, "hero").map_err(DeadlockError::from)?;
    let enemy_query: &str =
        required_option(&mut options, "vs").map_err(DeadlockError::from)?;
    let rank_query: Option<&str> = optional_option(&mut options, "rank");

    cx.interaction.defer(&cx.ctx.http).await?;

    let (hero, _) = resolve_hero(api, hero_query).await?;
    let (enemy, _) = resolve_hero(api, enemy_query).await?;
    if hero.id == enemy.id {
        return Err(DeadlockError::SameHero);
    }

    let rank = match rank_query {
        Some(query) => {
            let ranks = api.ranks(Priority::Interactive).await?;
            Some(resolve_rank(&ranks.value, query)?)
        },
        None => None,
    };
    let badge = rank.as_ref().map(|(range, _)| *range);
    let label = rank.as_ref().map(|(_, label)| label.as_str());

    let (matchups, curves) = futures::try_join!(
        api.lane_matchups(hero.id, enemy.id, badge, Priority::Interactive),
        api.lane_soul_curves(hero.id, enemy.id, badge, Priority::Interactive),
    )?;

    let lane = lane_summary(&matchups.value, &curves.value, hero.id, enemy.id);
    let fresh = Freshness::combine(&[
        Freshness { fetched_at: matchups.fetched_at, stale: matchups.stale },
        Freshness { fetched_at: curves.fetched_at, stale: curves.stale },
    ]);

    let component =
        embeds::matchup_component(&hero, &enemy, label, lane.as_ref(), &fresh);

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
