use std::sync::Arc;

use serenity::all::{CreateEmbed, CreateEmbedFooter, EditInteractionResponse};
use zayden_core::InvocationCtx;

use crate::embeds::COLOUR;
use crate::error::Result;
use crate::index::LibraryItemRow;
use crate::runtime::JellyfinRuntime;

pub async fn run(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
) -> Result<()> {
    cx.interaction.defer(&cx.ctx.http).await?;

    let counts = match runtime.caches.counts.get(&()).await {
        Some(cached) => cached,
        None => {
            let fetched = runtime.jellyfin.counts().await?;
            runtime.caches.counts.insert((), fetched).await;
            fetched
        },
    };

    let indexed_movies = LibraryItemRow::count(&cx.app.db, "Movie").await?;
    let indexed_series = LibraryItemRow::count(&cx.app.db, "Series").await?;

    let parties = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM jellyfin_parties WHERE state <> 'cancelled'"
    )
    .fetch_one(&cx.app.db)
    .await?
    .unwrap_or(0);

    let linked = sqlx::query_scalar!("SELECT COUNT(*) FROM jellyfin_users")
        .fetch_one(&cx.app.db)
        .await?
        .unwrap_or(0);

    let embed = CreateEmbed::new()
        .title("Media server")
        .colour(COLOUR)
        .field("Movies", counts.movies.to_string(), true)
        .field("Series", counts.series.to_string(), true)
        .field("Episodes", counts.episodes.to_string(), true)
        .field("Linked accounts", linked.to_string(), true)
        .field("Watch parties", parties.to_string(), true)
        .field("Indexed", format!("{indexed_movies} / {indexed_series}"), true)
        .footer(CreateEmbedFooter::new(
            "Indexed counts come from the local index, which refreshes every 6 hours.",
        ));

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}
