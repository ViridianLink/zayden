use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::config::{BotConfig, ConfigStore};
use crate::entitlement::EntitlementService;
use crate::events::AppEvent;

/// Shared application state for both the Discord bot and web backend.
///
/// Has no Serenity dependency — bot-specific caches live in `BotState`
/// (defined in `bot/src/state.rs`).
pub struct AppState {
    pub db: PgPool,
    pub config_store: Arc<ConfigStore>,
    pub entitlements: Arc<EntitlementService>,
    /// Cross-process broadcast bus (config invalidation, entitlement changes,
    /// …).
    pub events: broadcast::Sender<AppEvent>,
    pub http: reqwest::Client,
    /// AI provider API key (`OpenRouter` or any OpenAI-compatible provider).
    pub ai_provider_key: String,
    /// Base URL of the AI provider endpoint.
    pub ai_api_endpoint: String,
    /// Model identifier passed to the AI provider.
    pub ai_model: String,
    /// Google Sheets API key for endgame-analysis and destiny2 compendium.
    pub google_api_key: String,
    pub error_log_webhook: String,
    pub normal_log_webhook: String,
}

impl AppState {
    /// Construct `AppState` from an already-established pool and a loaded
    /// `BotConfig`.  `EntitlementService` remains a placeholder until M4.
    #[must_use]
    pub fn new(pool: PgPool, config: &BotConfig) -> Self {
        // 64-slot channel is plenty for current usage; resize when needed.
        let (events, _) = broadcast::channel(64);

        let config_store = Arc::new(ConfigStore::new(pool.clone(), events.clone()));
        ConfigStore::spawn_invalidator(
            Arc::clone(&config_store),
            events.subscribe(),
        );

        let entitlements =
            Arc::new(EntitlementService::new(pool.clone(), events.clone()));
        EntitlementService::spawn_invalidator(
            Arc::clone(&entitlements),
            events.subscribe(),
        );

        Self {
            db: pool,
            config_store,
            entitlements,
            events,
            http: reqwest::Client::new(),
            ai_provider_key: config.ai_provider_key.clone(),
            ai_api_endpoint: config.ai_api_endpoint.clone(),
            ai_model: config.ai_model.clone(),
            google_api_key: config.google_api_key.clone(),
            error_log_webhook: config.error_log_webhook.clone().unwrap_or_default(),
            normal_log_webhook: config
                .normal_log_webhook
                .clone()
                .unwrap_or_default(),
        }
    }

    /// Subscribe to the in-process event bus.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.events.subscribe()
    }
}
