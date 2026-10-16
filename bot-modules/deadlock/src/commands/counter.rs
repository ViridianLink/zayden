use std::collections::HashMap;

use deadlock_api::Priority;
use serenity::all::{EditInteractionResponse, MessageFlags, ResolvedValue};
use zayden_core::{InvocationCtx, required_option};

use super::resolve_hero;
use crate::analysis::counters;
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

    cx.interaction.defer(&cx.ctx.http).await?;

    let (hero, index) = resolve_hero(api, hero_query).await?;

    let rows = api.hero_counters(None, Priority::Interactive).await?;

    let shown = counters(&rows.value, hero.id);
    let fresh = Freshness::combine(&[Freshness {
        fetched_at: rows.fetched_at,
        stale: rows.stale,
    }]);

    let component = embeds::counter_component(&hero, &shown, &index, &fresh);

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
