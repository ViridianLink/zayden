use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use jiff::{Span, Zoned};
use serenity::all::{CreateEmbed, EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, optional_option, required_option};

use crate::discovery::binge::{self, format_duration};
use crate::discovery::resolve_local;
use crate::embeds::COLOUR;
use crate::error::Result;

const DEFAULT_PER_DAY: i64 = 2;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let show: &str = required_option(&mut options, "show")?;
    let per_day = optional_option::<i64, _>(&mut options, "per_day")
        .unwrap_or(DEFAULT_PER_DAY)
        .clamp(1, 10);
    let skip_intros =
        optional_option::<bool, _>(&mut options, "skip_intros").unwrap_or(true);

    cx.interaction.defer(&cx.ctx.http).await?;

    let series = resolve_local(&cx.app.db, show, Some("Series")).await?;
    let plan = binge::plan(runtime, &series.item_id, skip_intros).await?;

    let days = plan.days_at(usize::try_from(per_day).unwrap_or(1));
    let finish = Zoned::now() + Span::new().days(i64::try_from(days).unwrap_or(0));

    let mut embed = CreateEmbed::new()
        .title(format!("Binge plan: {}", series.name))
        .colour(COLOUR)
        .field("Episodes", plan.episodes.to_string(), true)
        .field("Total runtime", format_duration(plan.total_seconds), true)
        .field(
            "Average episode",
            format_duration(plan.average_episode_seconds()),
            true,
        );

    if skip_intros && plan.segments.has_coverage() {
        embed = embed.field(
            "Minus intros and credits",
            format_duration(plan.measured_seconds()),
            true,
        );
    }

    embed = embed
        .field(
            format!("At {per_day} a day"),
            format!("{days} days — finishing {}", finish.strftime("%d %B %Y")),
            false,
        )
        .field("Coverage", plan.coverage_note(), false);

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}
