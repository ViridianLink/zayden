use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use moka::future::Cache;
use sqlx::PgPool;

use super::command_modules;

pub type ModuleStates = HashMap<String, bool>;

const CACHE_TTL: Duration = Duration::from_secs(60);

pub struct ModuleStore {
    db: PgPool,
    cache: Cache<i64, Arc<ModuleStates>>,
}

impl ModuleStore {
    #[must_use]
    pub fn new(db: PgPool) -> Self {
        let cache =
            Cache::builder().max_capacity(1_024).time_to_live(CACHE_TTL).build();
        Self { db, cache }
    }

    pub async fn states(
        &self,
        guild_id: i64,
    ) -> Result<Arc<ModuleStates>, sqlx::Error> {
        if let Some(cached) = self.cache.get(&guild_id).await {
            return Ok(cached);
        }

        let states = sqlx::query!(
            "SELECT module, enabled FROM guild_modules WHERE guild_id = $1",
            guild_id
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|row| (row.module, row.enabled))
        .collect::<ModuleStates>();

        let states = Arc::new(states);
        self.cache.insert(guild_id, Arc::clone(&states)).await;
        Ok(states)
    }

    pub async fn refresh(
        &self,
        guild_id: i64,
    ) -> Result<Arc<ModuleStates>, sqlx::Error> {
        self.cache.invalidate(&guild_id).await;
        self.states(guild_id).await
    }

    pub async fn needs_import(&self, guild_id: i64) -> Result<bool, sqlx::Error> {
        let seeded = sqlx::query_scalar!(
            r#"SELECT bot_joined_at IS NOT NULL AS "seeded!" FROM guilds WHERE id = $1"#,
            guild_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(!seeded.unwrap_or(false))
    }

    pub async fn seed(
        &self,
        guild_id: i64,
        bot_joined_at: Timestamp,
        imported_disabled: &HashSet<&str>,
    ) -> Result<Arc<ModuleStates>, sqlx::Error> {
        let mut tx = self.db.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        #[expect(
            trivial_casts,
            reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
        )]
        let joined = sqlx::query!(
            r#"
            WITH previous AS (
                SELECT bot_joined_at FROM guilds WHERE id = $1 FOR UPDATE
            )
            UPDATE guilds
            SET bot_joined_at = $2
            FROM previous
            WHERE guilds.id = $1 AND previous.bot_joined_at IS DISTINCT FROM $2
            RETURNING previous.bot_joined_at AS "previous?: jiff_sqlx::Timestamp"
            "#,
            guild_id,
            bot_joined_at.to_sqlx() as jiff_sqlx::Timestamp,
        )
        .fetch_optional(&mut *tx)
        .await?;

        let first_seed = matches!(joined, Some(ref row) if row.previous.is_none());
        let rejoined = matches!(joined, Some(ref row) if row.previous.is_some());

        if rejoined {
            sqlx::query!("DELETE FROM guild_modules WHERE guild_id = $1", guild_id)
                .execute(&mut *tx)
                .await?;
        }

        let (modules, enabled): (Vec<String>, Vec<bool>) = command_modules()
            .map(|m| {
                let imported_off = first_seed && imported_disabled.contains(m.id);
                (m.id.to_owned(), m.default_enabled(bot_joined_at) && !imported_off)
            })
            .unzip();

        sqlx::query!(
            "INSERT INTO guild_modules (guild_id, module, enabled)
             SELECT $1, module, enabled FROM UNNEST($2::text[], $3::bool[]) AS t (module, enabled)
             ON CONFLICT (guild_id, module) DO NOTHING",
            guild_id,
            &modules,
            &enabled,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.refresh(guild_id).await
    }

    pub async fn set(
        &self,
        guild_id: i64,
        module: &str,
        enabled: bool,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.db.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO guild_modules (guild_id, module, enabled) VALUES ($1, $2, $3)
             ON CONFLICT (guild_id, module) DO UPDATE SET enabled = EXCLUDED.enabled",
            guild_id,
            module,
            enabled,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "SELECT pg_notify('modules_changed', $1)",
            guild_id.to_string()
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.cache.invalidate(&guild_id).await;
        Ok(())
    }
}
