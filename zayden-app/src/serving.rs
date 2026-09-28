use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use sqlx::PgPool;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::{self, Receiver};
use tracing::warn;

use crate::events::AppEvent;

const CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServingKind {
    Zayden,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServingBot {
    pub application_id: u64,
    pub kind: ServingKind,
    pub present: bool,
}

pub struct ServingResolver {
    db: PgPool,
    zayden_id: u64,
    cache: Cache<i64, ServingBot>,
}

impl ServingResolver {
    #[must_use]
    pub fn new(db: PgPool, zayden_id: u64) -> Self {
        let cache =
            Cache::builder().max_capacity(4_096).time_to_live(CACHE_TTL).build();
        Self { db, zayden_id, cache }
    }

    pub async fn resolve(&self, guild_id: i64) -> sqlx::Result<ServingBot> {
        if let Some(cached) = self.cache.get(&guild_id).await {
            return Ok(cached);
        }

        let row = sqlx::query!(
            r#"WITH s AS (SELECT serving_application($1) AS custom)
               SELECT s.custom,
                      EXISTS (
                          SELECT 1 FROM guild_presence p
                          WHERE p.guild_id = $1
                            AND p.application_id = COALESCE(s.custom, $2)
                            AND p.left_at IS NULL
                      ) AS "present!"
               FROM s"#,
            guild_id,
            self.zayden_id.cast_signed(),
        )
        .fetch_one(&self.db)
        .await?;

        let (application_id, kind) =
            row.custom.map_or((self.zayden_id, ServingKind::Zayden), |custom| {
                (custom.cast_unsigned(), ServingKind::Custom)
            });
        let bot = ServingBot { application_id, kind, present: row.present };

        self.cache.insert(guild_id, bot).await;
        Ok(bot)
    }

    pub fn spawn_invalidator(this: Arc<Self>, mut rx: Receiver<AppEvent>) {
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(AppEvent::ServingChanged(guild_id)) => {
                        this.cache.invalidate(&guild_id.cast_signed()).await;
                    },
                    Ok(AppEvent::CustomBotsChanged(_) | AppEvent::Resync) => {
                        this.cache.invalidate_all();
                    },
                    Ok(_) => {},
                    Err(RecvError::Lagged(n)) => {
                        warn!(n, "serving invalidator lagged; clearing full cache");
                        this.cache.invalidate_all();
                    },
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }
}
