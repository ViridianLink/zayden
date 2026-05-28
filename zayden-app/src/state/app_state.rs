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
}

impl AppState {
    /// Construct `AppState` from an already-established pool and a loaded
    /// `BotConfig`.  Placeholder services (`ConfigStore`, `EntitlementService`)
    /// are empty shells that will be wired up in M2/M4 respectively.
    pub fn new(pool: PgPool, _config: &BotConfig) -> Self {
        // 64-slot channel is plenty for the current placeholder; resize later.
        let (events, _) = broadcast::channel(64);

        Self {
            db: pool,
            config_store: Arc::new(ConfigStore),
            entitlements: Arc::new(EntitlementService),
            events,
            http: reqwest::Client::new(),
        }
    }

    /// Subscribe to the in-process event bus.
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.events.subscribe()
    }
}
