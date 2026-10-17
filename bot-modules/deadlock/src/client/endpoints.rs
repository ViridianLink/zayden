use deadlock_api::Priority;
use deadlock_api::model::{
    AbilityOrderStats,
    AccountRank,
    Build,
    Hero,
    HeroCounterStats,
    Item,
    ItemFlowStats,
    ItemStats,
    LaneMatchupStats,
    LaneSoulCurve,
    MatchMetadata,
    PatchNote,
    PerformancePoint,
    Rank,
};
use deadlock_api::rank::BadgeRange;
use jiff::Timestamp;

use super::{
    ANALYTICS_TTL,
    ASSET_TTL,
    BUILD_TTL,
    CacheKind,
    CachePolicy,
    DeadlockApi,
    Fetched,
    PATCH_TTL,
};
use crate::error::Result;

const ANALYTICS_POLICY: CachePolicy = CachePolicy {
    kind: CacheKind::Analytics,
    ttl: ANALYTICS_TTL,
    client_version: None,
};

fn badge_key(badge: Option<BadgeRange>) -> String {
    badge.map_or_else(
        || "any".to_owned(),
        |range| format!("{}-{}", range.min, range.max),
    )
}

impl DeadlockApi {
    pub async fn client_versions(
        &self,
        priority: Priority,
    ) -> Result<Fetched<Vec<u32>>> {
        let policy = CachePolicy {
            kind: CacheKind::Patch,
            ttl: PATCH_TTL,
            client_version: None,
        };

        self.cached(
            &self.versions_cache,
            "client_versions".to_owned(),
            policy,
            || self.client.client_versions(priority),
        )
        .await
    }

    pub async fn heroes(
        &self,
        version: Option<u32>,
        priority: Priority,
    ) -> Result<Fetched<Vec<Hero>>> {
        let policy = CachePolicy {
            kind: CacheKind::Asset,
            ttl: ASSET_TTL,
            client_version: version,
        };

        self.cached(&self.heroes_cache, "heroes".to_owned(), policy, || {
            self.client.heroes(version, priority)
        })
        .await
    }

    pub async fn items(
        &self,
        version: Option<u32>,
        priority: Priority,
    ) -> Result<Fetched<Vec<Item>>> {
        let policy = CachePolicy {
            kind: CacheKind::Asset,
            ttl: ASSET_TTL,
            client_version: version,
        };

        self.cached(&self.items_cache, "items".to_owned(), policy, || {
            self.client.items(version, priority)
        })
        .await
    }

    pub async fn ranks(&self, priority: Priority) -> Result<Fetched<Vec<Rank>>> {
        let policy = CachePolicy {
            kind: CacheKind::Asset,
            ttl: ASSET_TTL,
            client_version: None,
        };

        self.cached(&self.ranks_cache, "ranks".to_owned(), policy, || {
            self.client.ranks(priority)
        })
        .await
    }

    pub async fn patch_notes(
        &self,
        priority: Priority,
    ) -> Result<Fetched<Vec<PatchNote>>> {
        let policy = CachePolicy {
            kind: CacheKind::Patch,
            ttl: PATCH_TTL,
            client_version: None,
        };

        self.cached(&self.patches_cache, "patch_notes".to_owned(), policy, || {
            self.client.patch_notes(priority)
        })
        .await
    }

    pub async fn item_flow(
        &self,
        hero_id: u32,
        badge: Option<BadgeRange>,
        priority: Priority,
    ) -> Result<Fetched<ItemFlowStats>> {
        let key = format!("item_flow?hero={hero_id}&badge={}", badge_key(badge));

        self.cached(&self.item_flow_cache, key, ANALYTICS_POLICY, || {
            self.client.item_flow(hero_id, badge, priority)
        })
        .await
    }

    pub async fn ability_orders(
        &self,
        hero_id: u32,
        badge: Option<BadgeRange>,
        priority: Priority,
    ) -> Result<Fetched<Vec<AbilityOrderStats>>> {
        let key =
            format!("ability_orders?hero={hero_id}&badge={}", badge_key(badge));

        self.cached(&self.ability_orders_cache, key, ANALYTICS_POLICY, || {
            self.client.ability_orders(hero_id, badge, priority)
        })
        .await
    }

    pub async fn hero_counters(
        &self,
        badge: Option<BadgeRange>,
        priority: Priority,
    ) -> Result<Fetched<Vec<HeroCounterStats>>> {
        let key = format!("hero_counters?badge={}", badge_key(badge));

        self.cached(&self.hero_counters_cache, key, ANALYTICS_POLICY, || {
            self.client.hero_counters(badge, priority)
        })
        .await
    }

    pub async fn lane_matchups(
        &self,
        hero_id: u32,
        enemy_hero_id: u32,
        badge: Option<BadgeRange>,
        priority: Priority,
    ) -> Result<Fetched<Vec<LaneMatchupStats>>> {
        let key = format!(
            "lane_matchups?hero={hero_id}&enemy={enemy_hero_id}&badge={}",
            badge_key(badge)
        );

        self.cached(&self.lane_matchups_cache, key, ANALYTICS_POLICY, || {
            self.client.lane_matchups(hero_id, enemy_hero_id, badge, priority)
        })
        .await
    }

    pub async fn lane_soul_curves(
        &self,
        hero_id: u32,
        enemy_hero_id: u32,
        badge: Option<BadgeRange>,
        priority: Priority,
    ) -> Result<Fetched<Vec<LaneSoulCurve>>> {
        let key = format!(
            "lane_soul_curves?hero={hero_id}&enemy={enemy_hero_id}&badge={}",
            badge_key(badge)
        );

        self.cached(&self.lane_soul_curves_cache, key, ANALYTICS_POLICY, || {
            self.client.lane_soul_curves(hero_id, enemy_hero_id, badge, priority)
        })
        .await
    }

    pub async fn item_stats(
        &self,
        hero_id: u32,
        priority: Priority,
    ) -> Result<Fetched<Vec<ItemStats>>> {
        let key = format!("item_stats?hero={hero_id}");

        self.cached(&self.item_stats_cache, key, ANALYTICS_POLICY, || {
            self.client.item_stats(hero_id, None, priority)
        })
        .await
    }

    pub async fn performance_curve(
        &self,
        hero_id: u32,
        priority: Priority,
    ) -> Result<Fetched<Vec<PerformancePoint>>> {
        let key = format!("performance_curve?hero={hero_id}");

        self.cached(&self.performance_curve_cache, key, ANALYTICS_POLICY, || {
            self.client.performance_curve(hero_id, None, priority)
        })
        .await
    }

    pub async fn builds(
        &self,
        hero_id: u32,
        priority: Priority,
    ) -> Result<Fetched<Vec<Build>>> {
        let policy = CachePolicy {
            kind: CacheKind::Build,
            ttl: BUILD_TTL,
            client_version: None,
        };
        let key = format!("builds?hero={hero_id}");

        self.cached(&self.builds_cache, key, policy, || {
            self.client.builds(hero_id, priority)
        })
        .await
    }

    pub async fn player_ranks(
        &self,
        account_ids: &[u32],
    ) -> Result<Vec<AccountRank>> {
        Ok(self.client.player_ranks(account_ids, Priority::Background).await?)
    }

    pub async fn recent_matches(
        &self,
        account_ids: &[u32],
        since: Timestamp,
    ) -> Result<Vec<MatchMetadata>> {
        Ok(self
            .client
            .recent_matches(account_ids, since, Priority::Background)
            .await?)
    }
}
