use jiff::{SignedDuration, Timestamp};
use sqlx::PgPool;

use crate::db::gc;

// Expired rows are kept for a week: client/cache.rs serves them when deadlock-api is
// down.
pub const STALE_CACHE_GRACE: SignedDuration = SignedDuration::from_hours(24 * 7);
pub const RANK_SNAPSHOT_RETENTION: SignedDuration =
    SignedDuration::from_hours(24 * 180);
pub const MATCH_RETENTION: SignedDuration = SignedDuration::from_hours(24 * 90);
pub const POST_RETENTION: SignedDuration = SignedDuration::from_hours(24 * 90);
pub const KEPT_SNAPSHOTS: i64 = 5;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct GcOutcome {
    pub cache_rows: u64,
    pub rank_snapshots: u64,
    pub matches: u64,
    pub guild_posts: u64,
    pub briefings: u64,
    pub asset_snapshots: u64,
}

pub async fn run(pool: &PgPool, now: Timestamp) -> sqlx::Result<GcOutcome> {
    let before = |retention: SignedDuration| {
        now.checked_sub(retention).unwrap_or(Timestamp::MIN)
    };

    Ok(GcOutcome {
        cache_rows: gc::prune_cache(pool, before(STALE_CACHE_GRACE)).await?,
        rank_snapshots: gc::prune_rank_snapshots(
            pool,
            before(RANK_SNAPSHOT_RETENTION),
        )
        .await?,
        matches: gc::prune_matches(pool, before(MATCH_RETENTION)).await?,
        guild_posts: gc::prune_guild_posts(pool, before(POST_RETENTION)).await?,
        briefings: gc::prune_briefings(pool, before(POST_RETENTION)).await?,
        asset_snapshots: gc::prune_asset_snapshots(pool, KEPT_SNAPSHOTS).await?,
    })
}
