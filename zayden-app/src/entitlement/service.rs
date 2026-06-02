use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tracing::{info, warn};

use super::types::{EntitlementScope, Tier};
use crate::events::AppEvent;

/// Premium entitlement gate, backed by the `entitlements` table and an
/// in-memory cache.
pub struct EntitlementService {
    db: PgPool,
    cache: Cache<EntitlementScope, Tier>,
    events: broadcast::Sender<AppEvent>,
}

impl EntitlementService {
    #[must_use]
    pub fn new(db: PgPool, events: broadcast::Sender<AppEvent>) -> Self {
        let cache = Cache::builder()
            .max_capacity(4096)
            .time_to_live(Duration::from_mins(10))
            .build();
        Self { db, cache, events }
    }

    /// Return the highest active tier for the given user across all scopes.
    pub async fn user_tier(&self, user_id: u64) -> Tier {
        let scope = EntitlementScope::User(user_id);
        self.tier_for_scope(scope).await
    }

    /// Return the highest active tier for the given guild.
    pub async fn guild_tier(&self, guild_id: u64) -> Tier {
        let scope = EntitlementScope::Guild(guild_id);
        self.tier_for_scope(scope).await
    }

    /// Return `true` if the principal described by `scope` meets `required`.
    ///
    /// For `UserInGuild`, the effective tier is the maximum of the user and
    /// guild tiers.
    pub async fn allows(&self, scope: EntitlementScope, required: Tier) -> bool {
        if required == Tier::Free {
            return true;
        }
        let effective = match &scope {
            EntitlementScope::UserInGuild(user_id, guild_id) => {
                let u = self.user_tier(*user_id).await;
                let g = self.guild_tier(*guild_id).await;
                u.max(g)
            },
            EntitlementScope::User(_) | EntitlementScope::Guild(_) => {
                self.tier_for_scope(scope).await
            },
        };
        effective >= required
    }

    /// Write an entitlement row (or upsert if `external_id` already exists) and
    /// refresh the denormalised cache row.  Emits
    /// `AppEvent::EntitlementChanged` so other processes can evict their
    /// caches.
    pub async fn grant(
        &self,
        scope: EntitlementScope,
        tier: Tier,
        provider: &str,
        external_id: &str,
        expires_at: Option<jiff::Timestamp>,
    ) -> Result<(), sqlx::Error> {
        let expires_at_pg: Option<jiff_sqlx::Timestamp> = expires_at.map(Into::into);

        sqlx::query(
            r"
            INSERT INTO entitlements
                (provider, external_id, scope_type, scope_id, scope_secondary_id, tier, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (provider, external_id)
            DO UPDATE SET
                scope_type         = EXCLUDED.scope_type,
                scope_id           = EXCLUDED.scope_id,
                scope_secondary_id = EXCLUDED.scope_secondary_id,
                tier               = EXCLUDED.tier,
                expires_at         = EXCLUDED.expires_at,
                granted_at         = now()
            ",
        )
        .bind(provider)
        .bind(external_id)
        .bind(scope.scope_type())
        .bind(scope.scope_id())
        .bind(scope.scope_secondary_id())
        .bind(tier.as_str())
        .bind(expires_at_pg)
        .execute(&self.db)
        .await?;

        self.refresh_cache_row(&scope, tier).await?;
        self.cache.invalidate(&scope).await;
        let _ = self.events.send(AppEvent::EntitlementChanged(scope));
        Ok(())
    }

    /// Remove an entitlement by `provider` + `external_id` and refresh the
    /// cache row.
    pub async fn revoke(
        &self,
        provider: &str,
        external_id: &str,
    ) -> Result<(), sqlx::Error> {
        let row = sqlx::query_as::<_, (String, i64, i64)>(
            "DELETE FROM entitlements WHERE provider = $1 AND external_id = $2
             RETURNING scope_type, scope_id, scope_secondary_id",
        )
        .bind(provider)
        .bind(external_id)
        .fetch_optional(&self.db)
        .await?;

        if let Some((scope_type, scope_id, scope_secondary_id)) = row {
            let scope = row_to_scope(&scope_type, scope_id, scope_secondary_id);
            self.refresh_cache_row_from_db(&scope).await?;
            self.cache.invalidate(&scope).await;
            let _ = self.events.send(AppEvent::EntitlementChanged(scope));
        }
        Ok(())
    }

    /// Spawn a background task that evicts cache entries when it receives
    /// `AppEvent::EntitlementChanged` on the broadcast bus.
    pub fn spawn_invalidator(
        this: Arc<Self>,
        mut rx: broadcast::Receiver<AppEvent>,
    ) {
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(AppEvent::EntitlementChanged(scope)) => {
                        this.cache.invalidate(&scope).await;
                    },
                    Ok(_) => {},
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(
                            n,
                            "entitlement invalidator lagged; clearing full cache"
                        );
                        this.cache.invalidate_all();
                    },
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    // ── private helpers ──────────────────────────────────────────────────────

    async fn tier_for_scope(&self, scope: EntitlementScope) -> Tier {
        if let Some(tier) = self.cache.get(&scope).await {
            return tier;
        }
        match self.load_tier_from_db(&scope).await {
            Ok(tier) => {
                self.cache.insert(scope, tier).await;
                tier
            },
            Err(err) => {
                warn!(?err, "failed to load entitlement tier; defaulting to Free");
                Tier::Free
            },
        }
    }

    async fn load_tier_from_db(
        &self,
        scope: &EntitlementScope,
    ) -> Result<Tier, sqlx::Error> {
        // Check the denormalised cache row first.
        let row = sqlx::query_as::<_, (String,)>(
            r"
            SELECT tier FROM entitlement_cache
            WHERE scope_type = $1
              AND scope_id = $2
              AND scope_secondary_id = $3
            ",
        )
        .bind(scope.scope_type())
        .bind(scope.scope_id())
        .bind(scope.scope_secondary_id())
        .fetch_optional(&self.db)
        .await?;

        if let Some((tier_str,)) = row {
            return Ok(tier_str.parse().unwrap_or(Tier::Free));
        }

        // No cache row — compute from entitlements table.
        self.aggregate_tier_from_db(scope).await
    }

    async fn aggregate_tier_from_db(
        &self,
        scope: &EntitlementScope,
    ) -> Result<Tier, sqlx::Error> {
        let row = sqlx::query_as::<_, (Option<i32>,)>(
            r"
            SELECT MAX(
                CASE tier
                    WHEN 'enterprise' THEN 2
                    WHEN 'pro'        THEN 1
                    ELSE                   0
                END
            ) AS max_tier
            FROM entitlements
            WHERE scope_type = $1
              AND scope_id = $2
              AND scope_secondary_id = $3
              AND (expires_at IS NULL OR expires_at > now())
            ",
        )
        .bind(scope.scope_type())
        .bind(scope.scope_id())
        .bind(scope.scope_secondary_id())
        .fetch_one(&self.db)
        .await?;

        let tier = match row.0 {
            Some(2) => Tier::Enterprise,
            Some(1) => Tier::Pro,
            _ => Tier::Free,
        };
        Ok(tier)
    }

    async fn refresh_cache_row(
        &self,
        scope: &EntitlementScope,
        tier: Tier,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO entitlement_cache
                (scope_type, scope_id, scope_secondary_id, tier, refreshed_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (scope_type, scope_id, scope_secondary_id)
            DO UPDATE SET tier = EXCLUDED.tier, refreshed_at = now()
            ",
        )
        .bind(scope.scope_type())
        .bind(scope.scope_id())
        .bind(scope.scope_secondary_id())
        .bind(tier.as_str())
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn refresh_cache_row_from_db(
        &self,
        scope: &EntitlementScope,
    ) -> Result<(), sqlx::Error> {
        let tier = self.aggregate_tier_from_db(scope).await?;
        self.refresh_cache_row(scope, tier).await?;
        info!(
            scope_type = scope.scope_type(),
            scope_id = scope.scope_id(),
            tier = tier.as_str(),
            "entitlement cache refreshed after revoke",
        );
        Ok(())
    }
}

fn row_to_scope(
    scope_type: &str,
    scope_id: i64,
    scope_secondary_id: i64,
) -> EntitlementScope {
    match scope_type {
        "guild" => EntitlementScope::Guild(u64::try_from(scope_id).unwrap_or(0)),
        "user_in_guild" => EntitlementScope::UserInGuild(
            u64::try_from(scope_id).unwrap_or(0),
            u64::try_from(scope_secondary_id).unwrap_or(0),
        ),
        _ => EntitlementScope::User(u64::try_from(scope_id).unwrap_or(0)),
    }
}
