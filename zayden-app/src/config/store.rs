use std::sync::Arc;

use moka::future::Cache;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tracing::warn;

use crate::events::AppEvent;

use super::guild_config::{GuildConfig, GuildConfigPatch, ModuleConfig};

/// Single read/write entry point for guild configuration.
///
/// Caches `GuildConfig` in-memory (moka async cache) and emits
/// `AppEvent::ConfigChanged` on every write so all subscribers can evict.
pub struct ConfigStore {
    db: PgPool,
    cache: Cache<i64, Arc<GuildConfig>>,
    events: broadcast::Sender<AppEvent>,
}

impl ConfigStore {
    pub fn new(db: PgPool, events: broadcast::Sender<AppEvent>) -> Self {
        let cache = Cache::builder().max_capacity(1_024).build();
        Self { db, cache, events }
    }

    /// Construct a transient `ConfigStore` from a bare pool.
    ///
    /// Used by binding-layer trait impls that only receive a `&PgPool`.  The
    /// resulting store has its own short-lived cache and a disconnected event
    /// channel; it does NOT share the application-level cache in `AppState`.
    /// This is a stepping-stone: M3 will thread `Arc<ConfigStore>` directly
    /// through command handlers.
    pub fn from_pool(db: PgPool) -> Self {
        let (events, _) = broadcast::channel(1);
        Self::new(db, events)
    }

    /// Return the cached `GuildConfig` for `guild_id`, returning `None` on a
    /// cache miss where the row does not exist.
    pub async fn try_get(&self, guild_id: i64) -> Result<Option<Arc<GuildConfig>>, sqlx::Error> {
        if let Some(cached) = self.cache.get(&guild_id).await {
            return Ok(Some(cached));
        }

        let row: Option<GuildConfig> = sqlx::query_as(
            r#"
            SELECT
                id,
                support_channel_id,
                support_thread_id,
                support_role_id,
                faq_channel_id,
                suggestions_channel_id,
                review_channel_id,
                rules_channel_id,
                general_channel_id,
                spoiler_channel_id,
                artist_role_id,
                sleep_role_id,
                temp_voice_category,
                temp_voice_creator_channel,
                thread_id,
                lfg_channel_id,
                lfg_role_id,
                lfg_scheduled_thread_id,
                updated_at
            FROM guild_config
            WHERE id = $1
            "#,
        )
        .bind(guild_id)
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(r) => {
                let entry = Arc::new(r);
                self.cache.insert(guild_id, Arc::clone(&entry)).await;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Return the cached `GuildConfig` for `guild_id`, loading from DB on miss.
    pub async fn get(&self, guild_id: i64) -> Result<Arc<GuildConfig>, sqlx::Error> {
        self.try_get(guild_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Apply a patch to the guild config, persist it, invalidate the cache, and
    /// emit `AppEvent::ConfigChanged`.
    ///
    /// Upserts: if no row exists for `guild_id` yet, one is created with
    /// default/null values before the COALESCE-based UPDATE runs.
    pub async fn update<F>(&self, guild_id: i64, f: F) -> Result<Arc<GuildConfig>, sqlx::Error>
    where
        F: FnOnce(&mut GuildConfigPatch),
    {
        let mut patch = GuildConfigPatch::default();
        f(&mut patch);

        sqlx::query("INSERT INTO guild_config (id) VALUES ($1) ON CONFLICT (id) DO NOTHING")
            .bind(guild_id)
            .execute(&self.db)
            .await?;

        sqlx::query!(
            r#"
            UPDATE guild_config SET
                support_channel_id          = COALESCE($2,  support_channel_id),
                support_thread_id           = COALESCE($3,  support_thread_id),
                support_role_id             = COALESCE($4,  support_role_id),
                faq_channel_id              = COALESCE($5,  faq_channel_id),
                suggestions_channel_id      = COALESCE($6,  suggestions_channel_id),
                review_channel_id           = COALESCE($7,  review_channel_id),
                rules_channel_id            = COALESCE($8,  rules_channel_id),
                general_channel_id          = COALESCE($9,  general_channel_id),
                spoiler_channel_id          = COALESCE($10, spoiler_channel_id),
                artist_role_id              = COALESCE($11, artist_role_id),
                sleep_role_id               = COALESCE($12, sleep_role_id),
                temp_voice_category         = COALESCE($13, temp_voice_category),
                temp_voice_creator_channel  = COALESCE($14, temp_voice_creator_channel),
                thread_id                   = COALESCE($15, thread_id),
                lfg_channel_id              = COALESCE($16, lfg_channel_id),
                lfg_role_id                 = COALESCE($17, lfg_role_id),
                lfg_scheduled_thread_id     = COALESCE($18, lfg_scheduled_thread_id),
                updated_at                  = now()
            WHERE id = $1
            "#,
            guild_id,
            patch.support_channel_id,
            patch.support_thread_id,
            patch.support_role_id,
            patch.faq_channel_id,
            patch.suggestions_channel_id,
            patch.review_channel_id,
            patch.rules_channel_id,
            patch.general_channel_id,
            patch.spoiler_channel_id,
            patch.artist_role_id,
            patch.sleep_role_id,
            patch.temp_voice_category,
            patch.temp_voice_creator_channel,
            patch.thread_id,
            patch.lfg_channel_id,
            patch.lfg_role_id,
            patch.lfg_scheduled_thread_id
        )
        .execute(&self.db)
        .await?;

        self.cache.invalidate(&guild_id).await;
        let _ = self.events.send(AppEvent::ConfigChanged(guild_id as u64));

        self.get(guild_id).await
    }

    /// Hydrate a module-specific config from `guild_settings_kv`.
    pub async fn module<M: ModuleConfig>(&self, guild_id: i64) -> Result<M, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, serde_json::Value)>(
            r#"
            SELECT key, value
            FROM guild_settings_kv
            WHERE guild_id = $1 AND module = $2
            "#,
        )
        .bind(guild_id)
        .bind(M::module_name())
        .fetch_all(&self.db)
        .await?;

        let kv: std::collections::HashMap<String, serde_json::Value> = rows.into_iter().collect();

        Ok(M::from_kv(&kv))
    }

    /// Persist a module-specific config to `guild_settings_kv`, invalidating
    /// the guild's cache entry and emitting `AppEvent::ConfigChanged`.
    pub async fn set_module<M: ModuleConfig>(
        &self,
        guild_id: i64,
        value: &M,
    ) -> Result<(), sqlx::Error> {
        let kv = value.to_kv();
        let module = M::module_name();

        for (key, val) in &kv {
            sqlx::query!(
                r#"
                INSERT INTO guild_settings_kv (guild_id, module, key, value, updated_at)
                VALUES ($1, $2, $3, $4, now())
                ON CONFLICT (guild_id, module, key)
                DO UPDATE SET value = EXCLUDED.value, updated_at = now()
                "#,
                guild_id,
                module,
                key,
                val
            )
            .execute(&self.db)
            .await?;
        }

        self.cache.invalidate(&guild_id).await;
        let _ = self.events.send(AppEvent::ConfigChanged(guild_id as u64));

        Ok(())
    }

    /// Spawn a background task that evicts cache entries on `AppEvent::ConfigChanged`.
    ///
    /// This is the cross-process invalidation path: when another OS process
    /// writes a guild config, the `ConfigListener` Postgres LISTEN task emits
    /// `ConfigChanged` onto the broadcast, which this task receives and acts on.
    pub fn spawn_invalidator(store: Arc<Self>, mut rx: broadcast::Receiver<AppEvent>) {
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(AppEvent::ConfigChanged(guild_id)) => {
                        store.cache.invalidate(&(guild_id as i64)).await;
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(
                            "ConfigStore invalidator lagged by {n} events; evicting all cache entries"
                        );
                        store.cache.invalidate_all();
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    /// Atomically increment `thread_id` for a guild, then invalidate its
    /// cache entry.  Used by the ticket module's sequential ticket counter.
    pub async fn increment_thread_id(&self, guild_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE guild_config SET thread_id = thread_id + 1 WHERE id = $1")
            .bind(guild_id)
            .execute(&self.db)
            .await?;

        self.cache.invalidate(&guild_id).await;
        let _ = self.events.send(AppEvent::ConfigChanged(guild_id as u64));

        Ok(())
    }
}
