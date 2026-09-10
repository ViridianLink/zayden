use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, required_option};

use crate::discovery::resolve;
use crate::embeds;
use crate::error::Result;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let first: &str = required_option(&mut options, "first")?;
    let second: &str = required_option(&mut options, "second")?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let a = resolve(runtime, &cx.app.db, first).await?;
    let b = resolve(runtime, &cx.app.db, second).await?;

    let a_id =
        a.tmdb_id().ok_or_else(|| JellyfinError::NoSuchTitle(first.to_owned()))?;
    let b_id =
        b.tmdb_id().ok_or_else(|| JellyfinError::NoSuchTitle(second.to_owned()))?;

    let (left, right) = futures::join!(
        runtime.seer.recommendations(a.kind(), a_id),
        runtime.seer.recommendations(b.kind(), b_id),
    );

    let left = left.map_err(JellyfinError::from)?;
    let right = right.map_err(JellyfinError::from)?;

    // The overlap is the point: a title both films recommend shares DNA with
    // both, which neither list alone tells you.
    let mut shared: Vec<_> = left
        .into_iter()
        .filter(|item| right.iter().any(|other| other.id == item.id))
        .collect();

    let note = if shared.is_empty() {
        shared = right;
        Some("Nothing sits between both, so here is what the second one suggests.")
    } else {
        None
    };

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().embed(embeds::media::list(
                &format!("Between {} and {}", a.title(), b.title()),
                &shared,
                note,
            )),
        )
        .await?;

    Ok(())
}
