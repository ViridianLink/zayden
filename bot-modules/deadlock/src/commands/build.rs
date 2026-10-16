use std::collections::HashMap;

use deadlock_api::Priority;
use serenity::all::{EditInteractionResponse, MessageFlags, ResolvedValue};
use zayden_core::{InvocationCtx, optional_option, required_option};

use super::{resolve_hero, resolve_rank};
use crate::analysis::item_phases;
use crate::client::DeadlockApi;
use crate::embeds::{self, Freshness};
use crate::error::{DeadlockError, Result};
use crate::items::ItemIndex;

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    api: &DeadlockApi,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let hero_query: &str =
        required_option(&mut options, "hero").map_err(DeadlockError::from)?;
    let rank_query: Option<&str> = optional_option(&mut options, "rank");

    cx.interaction.defer(&cx.ctx.http).await?;

    let (hero, _) = resolve_hero(api, hero_query).await?;
    let rank = match rank_query {
        Some(query) => {
            let ranks = api.ranks(Priority::Interactive).await?;
            Some(resolve_rank(&ranks.value, query)?)
        },
        None => None,
    };
    let badge = rank.as_ref().map(|(range, _)| *range);
    let label = rank.as_ref().map(|(_, label)| label.as_str());

    let (flow, items, builds) = futures::try_join!(
        api.item_flow(hero.id, badge, Priority::Interactive),
        api.items(None, Priority::Interactive),
        async {
            let builds =
                api.builds(hero.id, Priority::Interactive).await.map_or_else(
                    |error| {
                        tracing::warn!(
                            %error,
                            hero_id = hero.id,
                            "community builds unavailable"
                        );
                        None
                    },
                    Some,
                );
            Ok::<_, DeadlockError>(builds)
        },
    )?;

    let phases = item_phases(&flow.value);
    let item_index = ItemIndex::new(&items.value);
    let top_build = builds
        .as_ref()
        .and_then(|builds| builds.value.first())
        .map(|build| &build.hero_build);

    let mut sources = vec![
        Freshness { fetched_at: flow.fetched_at, stale: flow.stale },
        Freshness { fetched_at: items.fetched_at, stale: items.stale },
    ];
    if let Some(builds) = &builds {
        sources
            .push(Freshness { fetched_at: builds.fetched_at, stale: builds.stale });
    }
    let fresh = Freshness::combine(&sources);

    let component = embeds::build_component(
        &hero,
        label,
        &phases,
        top_build,
        &item_index,
        &fresh,
    );

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
