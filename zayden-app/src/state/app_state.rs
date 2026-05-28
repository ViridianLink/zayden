use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::{
    config::{BotConfig, ConfigStore},
    entitlement::EntitlementService,
    events::AppEvent,
};

/// Shared application state for both the Discord bot and web backend.
///
/// Has no Serenity dependency — bot-specific caches live in `BotState`
/// (defined in `bot/src/state.rs`).
pub struct AppState {
    pub db: PgPool,
    pub config_store: Arc<ConfigStore>,
    pub entitlements: Arc<EntitlementService>,
    /// Cross-process broadcast bus (config invalidation, entitlement changes, …).
    pub events: broadcast::Sender<AppEvent>,
    pub http: reqwest::Client,
    pub openai_api_key: String,
}

impl AppState {
    /// Construct `AppState` from an already-established pool and a loaded
    /// `BotConfig`.  `EntitlementService` remains a placeholder until M4.
    pub fn new(pool: PgPool, config: &BotConfig) -> Self {
        // 64-slot channel is plenty for current usage; resize when needed.
        let (events, _) = broadcast::channel(64);

        let config_store = Arc::new(ConfigStore::new(pool.clone(), events.clone()));
        ConfigStore::spawn_invalidator(Arc::clone(&config_store), events.subscribe());

        Self {
            db: pool,
            config_store,
            entitlements: Arc::new(EntitlementService),
            events,
            http: reqwest::Client::new(),
            openai_api_key: config.openai_api_key.clone(),
        }
    }

    /// Subscribe to the in-process event bus.
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.events.subscribe()
    }
}
