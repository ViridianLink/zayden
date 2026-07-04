use std::future::Future;
use std::sync::Arc;

use moka::future::Cache;
use sqlx::PgPool;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::warn;

use crate::events::AppEvent;

pub trait SettingsRow: Sized + Send + Sync + Clone + 'static {
    const TABLE: &'static str;

    fn empty(guild_id: i64) -> Self;

    fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> impl Future<Output = Result<Option<Self>, sqlx::Error>> + Send;

    fn upsert(
        &self,
        pool: &PgPool,
    ) -> impl Future<Output = Result<Self, sqlx::Error>> + Send;
}

pub struct SettingsStore<Row: SettingsRow> {
    db: PgPool,
    cache: Cache<i64, Arc<Row>>,
    events: Sender<AppEvent>,
}

impl<Row: SettingsRow> SettingsStore<Row> {
    #[must_use]
    pub fn new(db: PgPool, events: Sender<AppEvent>) -> Self {
        let cache = Cache::builder().max_capacity(1_024).build();
        Self { db, cache, events }
    }

    pub async fn try_get(
        &self,
        guild_id: i64,
    ) -> Result<Option<Arc<Row>>, sqlx::Error> {
        if let Some(cached) = self.cache.get(&guild_id).await {
            return Ok(Some(cached));
        }

        match Row::select(&self.db, guild_id).await? {
            Some(row) => {
                let entry = Arc::new(row);
                self.cache.insert(guild_id, Arc::clone(&entry)).await;
                Ok(Some(entry))
            },
            None => Ok(None),
        }
    }

    pub async fn get(&self, guild_id: i64) -> Result<Arc<Row>, sqlx::Error> {
        Ok(self
            .try_get(guild_id)
            .await?
            .unwrap_or_else(|| Arc::new(Row::empty(guild_id))))
    }

    pub async fn update<F>(
        &self,
        guild_id: i64,
        f: F,
    ) -> Result<Arc<Row>, sqlx::Error>
    where
        F: FnOnce(&mut Row),
    {
        let mut row = self
            .try_get(guild_id)
            .await?
            .map_or_else(|| Row::empty(guild_id), |arc| (*arc).clone());
        f(&mut row);

        let saved = row.upsert(&self.db).await?;

        let entry = Arc::new(saved);
        self.cache.insert(guild_id, Arc::clone(&entry)).await;
        let _ = self
            .events
            .send(AppEvent::ConfigChanged(u64::try_from(guild_id).unwrap_or(0)));

        Ok(entry)
    }

    pub fn spawn_invalidator(store: Arc<Self>, mut rx: Receiver<AppEvent>) {
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(AppEvent::ConfigChanged(guild_id)) => {
                        store
                            .cache
                            .invalidate(&i64::try_from(guild_id).unwrap_or(i64::MAX))
                            .await;
                    },
                    Ok(_) => {},
                    Err(RecvError::Lagged(n)) => {
                        warn!(
                            table = Row::TABLE,
                            n,
                            "SettingsStore invalidator lagged; evicting full cache"
                        );
                        store.cache.invalidate_all();
                    },
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }
}
