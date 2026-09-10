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
    let title: &str = required_option(&mut options, "title")?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let resolved = resolve(runtime, &cx.app.db, title).await?;
    let tmdb_id = resolved
        .tmdb_id()
        .ok_or_else(|| JellyfinError::NoSuchTitle(title.to_owned()))?;

    let results = runtime
        .seer
        .recommendations(resolved.kind(), tmdb_id)
        .await
        .map_err(JellyfinError::from)?;

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().embed(embeds::media::list(
                &format!("Because you watched {}", resolved.title()),
                &results,
                None,
            )),
        )
        .await?;

    Ok(())
}
