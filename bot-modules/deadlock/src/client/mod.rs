mod cache;
mod endpoints;

use std::sync::Arc;
use std::time::Duration;

use deadlock_api::DeadlockClient;
use deadlock_api::model::{
    AbilityOrderStats,
    Build,
    Hero,
    HeroCounterStats,
    Item,
    ItemFlowStats,
    ItemStats,
    LaneMatchupStats,
    LaneSoulCurve,
    PatchNote,
    PerformancePoint,
    Rank,
};
use moka::future::Cache;
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;

use crate::error::Result;

const ASSET_TTL: Duration = Duration::from_hours(24);
const ANALYTICS_TTL: Duration = Duration::from_hours(6);
const BUILD_TTL: Duration = Duration::from_hours(6);
const PATCH_TTL: Duration = Duration::from_mins(5);

const ASSET_CAPACITY: u64 = 16;
const ANALYTICS_CAPACITY: u64 = 2_000;
const BUILD_CAPACITY: u64 = 200;
const PATCH_CAPACITY: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheKind {
    Asset,
    Analytics,
    Build,
    Patch,
}

impl CacheKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Asset => "asset",
            Self::Analytics => "analytics",
            Self::Build => "build",
            Self::Patch => "patch",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CachePolicy {
    pub kind: CacheKind,
    pub ttl: Duration,
    pub client_version: Option<u32>,
}

#[derive(Debug)]
pub struct Fetched<T> {
    pub value: Arc<T>,
    pub fetched_at: jiff::Timestamp,
    pub stale: bool,
}

impl<T> Clone for Fetched<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            fetched_at: self.fetched_at,
            stale: self.stale,
        }
    }
}

fn l1_cache<T>(ttl: Duration, max_capacity: u64) -> Cache<String, Fetched<T>>
where
    T: Send + Sync + 'static,
{
    Cache::builder().time_to_live(ttl).max_capacity(max_capacity).build()
}

pub struct DeadlockApi {
    client: DeadlockClient,
    pool: PgPool,

    versions_cache: Cache<String, Fetched<Vec<u32>>>,
    heroes_cache: Cache<String, Fetched<Vec<Hero>>>,
    items_cache: Cache<String, Fetched<Vec<Item>>>,
    ranks_cache: Cache<String, Fetched<Vec<Rank>>>,
    patches_cache: Cache<String, Fetched<Vec<PatchNote>>>,
    item_flow_cache: Cache<String, Fetched<ItemFlowStats>>,
    ability_orders_cache: Cache<String, Fetched<Vec<AbilityOrderStats>>>,
    hero_counters_cache: Cache<String, Fetched<Vec<HeroCounterStats>>>,
    lane_matchups_cache: Cache<String, Fetched<Vec<LaneMatchupStats>>>,
    lane_soul_curves_cache: Cache<String, Fetched<Vec<LaneSoulCurve>>>,
    item_stats_cache: Cache<String, Fetched<Vec<ItemStats>>>,
    performance_curve_cache: Cache<String, Fetched<Vec<PerformancePoint>>>,
    builds_cache: Cache<String, Fetched<Vec<Build>>>,
}

impl DeadlockApi {
    pub fn new(
        http: reqwest::Client,
        pool: PgPool,
        api_key: Option<&SecretString>,
    ) -> Result<Self> {
        let mut builder = DeadlockClient::builder().http(http);

        if let Some(key) = api_key {
            builder = builder.api_key(key.expose_secret());
        }

        Ok(Self::from_client(builder.build()?, pool))
    }

    #[must_use]
    pub fn from_client(client: DeadlockClient, pool: PgPool) -> Self {
        Self {
            client,
            pool,
            versions_cache: l1_cache(PATCH_TTL, PATCH_CAPACITY),
            heroes_cache: l1_cache(ASSET_TTL, ASSET_CAPACITY),
            items_cache: l1_cache(ASSET_TTL, ASSET_CAPACITY),
            ranks_cache: l1_cache(ASSET_TTL, ASSET_CAPACITY),
            patches_cache: l1_cache(PATCH_TTL, PATCH_CAPACITY),
            item_flow_cache: l1_cache(ANALYTICS_TTL, ANALYTICS_CAPACITY),
            ability_orders_cache: l1_cache(ANALYTICS_TTL, ANALYTICS_CAPACITY),
            hero_counters_cache: l1_cache(ANALYTICS_TTL, ANALYTICS_CAPACITY),
            lane_matchups_cache: l1_cache(ANALYTICS_TTL, ANALYTICS_CAPACITY),
            lane_soul_curves_cache: l1_cache(ANALYTICS_TTL, ANALYTICS_CAPACITY),
            item_stats_cache: l1_cache(ANALYTICS_TTL, BUILD_CAPACITY),
            performance_curve_cache: l1_cache(ANALYTICS_TTL, BUILD_CAPACITY),
            builds_cache: l1_cache(BUILD_TTL, BUILD_CAPACITY),
        }
    }

    pub async fn invalidate_unversioned(&self) -> Result<()> {
        self.versions_cache.invalidate_all();
        self.heroes_cache.invalidate_all();
        self.items_cache.invalidate_all();
        self.ranks_cache.invalidate_all();
        self.patches_cache.invalidate_all();
        self.item_flow_cache.invalidate_all();
        self.ability_orders_cache.invalidate_all();
        self.hero_counters_cache.invalidate_all();
        self.lane_matchups_cache.invalidate_all();
        self.lane_soul_curves_cache.invalidate_all();
        self.item_stats_cache.invalidate_all();
        self.performance_curve_cache.invalidate_all();
        self.builds_cache.invalidate_all();

        sqlx::query!("DELETE FROM deadlock_api_cache WHERE client_version IS NULL")
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
