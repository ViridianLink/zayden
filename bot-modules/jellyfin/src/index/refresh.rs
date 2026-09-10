use std::sync::Arc;

use jiff::Timestamp;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::error::Result;
use crate::index::row::LibraryItemRow;
use crate::runtime::JellyfinRuntime;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefreshReport {
    pub movies: usize,
    pub series: usize,
    pub pruned: u64,
}

pub async fn refresh(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
) -> Result<RefreshReport> {
    let started_at = Timestamp::now();

    let movies =
        ingest(runtime, pool, runtime.jellyfin.movie_library_id(), "Movie").await?;
    let series =
        ingest(runtime, pool, runtime.jellyfin.show_library_id(), "Series").await?;
    let pruned = LibraryItemRow::prune_stale(pool, started_at).await?;

    let report = RefreshReport { movies, series, pruned };

    info!(
        movies = report.movies,
        series = report.series,
        pruned = report.pruned,
        "jellyfin library index refreshed"
    );

    Ok(report)
}

async fn ingest(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    parent_id: &str,
    item_type: &str,
) -> Result<usize> {
    let items = runtime.jellyfin.library_items(parent_id, item_type).await?;
    let mut written = 0;

    for item in &items {
        let row = LibraryItemRow::from_item(item, item_type);

        match row.upsert(pool).await {
            Ok(()) => written += 1,
            Err(e) => {
                warn!(error = ?e, item = %item.name, "could not index item");
            },
        }
    }

    Ok(written)
}
