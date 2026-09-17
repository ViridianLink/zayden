//! Regression tests for the jellyfin library index upsert.
//!
//! `jellyfin_library_items` has two keys: the `item_id` primary key and a partial
//! unique index on `(item_type, tmdb_id)`. [`LibraryItemRow::upsert`] only carried
//! an `ON CONFLICT (item_id)` arm, so when Jellyfin minted a fresh `item_id` for a
//! title whose files had been replaced, the stale row still held that title's slot
//! in the tmdb index and the insert aborted with `23505`. The refresh loop logged
//! `could not index item` and dropped the title for that cycle:
//!
//! ```text
//! duplicate key value violates unique constraint "jellyfin_library_items_tmdb_idx"
//! Key (item_type, tmdb_id)=(Movie, 687163) already exists.
//! ```
//!
//! The fix evicts any other row holding the same `(item_type, tmdb_id)` inside the
//! upsert's transaction. `reindexes_title_when_item_id_changes` is the fails-before
//! case; the other two pin the scope of that eviction so it cannot delete rows it
//! has no claim on.

use jellyfin::LibraryItemRow;
use sqlx::PgPool;

fn row(
    item_id: &str,
    item_type: &str,
    name: &str,
    tmdb_id: Option<i32>,
) -> LibraryItemRow {
    LibraryItemRow {
        item_id: item_id.to_owned(),
        item_type: item_type.to_owned(),
        name: name.to_owned(),
        sort_name: name.to_lowercase(),
        production_year: Some(2026),
        tmdb_id,
        imdb_id: None,
        tvdb_id: None,
        runtime_ticks: None,
        community_rating: None,
        genres: Vec::new(),
        parent_path: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn reindexes_title_when_item_id_changes(pool: PgPool) -> sqlx::Result<()> {
    row("old-id", "Movie", "Project Hail Mary", Some(687_163)).upsert(&pool).await?;

    row("new-id", "Movie", "Project Hail Mary", Some(687_163)).upsert(&pool).await?;

    let found = LibraryItemRow::by_tmdb(&pool, "Movie", 687_163).await?;

    assert_eq!(found.map(|r| r.item_id).as_deref(), Some("new-id"));
    assert_eq!(LibraryItemRow::count(&pool, "Movie").await?, 1);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn keeps_untagged_items_apart(pool: PgPool) -> sqlx::Result<()> {
    row("a", "Movie", "Home Video A", None).upsert(&pool).await?;
    row("b", "Movie", "Home Video B", None).upsert(&pool).await?;

    assert_eq!(LibraryItemRow::count(&pool, "Movie").await?, 2);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn keeps_shared_tmdb_id_across_item_types(pool: PgPool) -> sqlx::Result<()> {
    row("film", "Movie", "Shogun", Some(1_234)).upsert(&pool).await?;
    row("show", "Series", "Shogun", Some(1_234)).upsert(&pool).await?;

    assert_eq!(LibraryItemRow::count(&pool, "Movie").await?, 1);
    assert_eq!(LibraryItemRow::count(&pool, "Series").await?, 1);

    Ok(())
}
