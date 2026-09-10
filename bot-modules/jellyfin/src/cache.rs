use std::time::Duration;

use moka::future::Cache;

use crate::identity::quick_connect::PendingLink;
use crate::transport::jellyfin::model::ItemCounts;
use crate::transport::jellyseerr::model::{
    MovieDetails,
    RegionProviders,
    SearchResult,
    TvDetails,
};
use crate::transport::playback::model::{ItemPlayCount, RecentPlay};
use crate::transport::segments::SegmentStats;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;

#[derive(Debug, Clone)]
pub struct JellyfinCaches {
    pub pending_links: Cache<u64, PendingLink>,
    pub recent_plays: Cache<String, Vec<RecentPlay>>,
    pub play_counts: Cache<(), Vec<ItemPlayCount>>,
    pub movie_details: Cache<i32, MovieDetails>,
    pub tv_details: Cache<i32, TvDetails>,
    pub keyword_ids: Cache<String, Option<i32>>,
    pub watch_providers: Cache<(String, i32), Vec<RegionProviders>>,
    pub search: Cache<String, Vec<SearchResult>>,
    pub segment_stats: Cache<String, SegmentStats>,
    pub counts: Cache<(), ItemCounts>,
}

impl Default for JellyfinCaches {
    fn default() -> Self {
        Self::new()
    }
}

impl JellyfinCaches {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending_links: build(64, 6 * MINUTE),
            recent_plays: build(256, 15 * MINUTE),
            play_counts: build(1, 6 * HOUR),
            movie_details: build(2048, 6 * HOUR),
            tv_details: build(2048, 6 * HOUR),
            keyword_ids: build(512, 24 * HOUR),
            watch_providers: build(1024, 12 * HOUR),
            search: build(512, MINUTE),
            segment_stats: build(256, 24 * HOUR),
            counts: build(1, 10 * MINUTE),
        }
    }
}

fn build<K, V>(capacity: u64, ttl_secs: u64) -> Cache<K, V>
where
    K: std::hash::Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    Cache::builder()
        .max_capacity(capacity)
        .time_to_live(Duration::from_secs(ttl_secs))
        .build()
}
