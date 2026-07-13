use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;

use crate::config::{BotConfig, SettingsRegistry};
use crate::entitlement::EntitlementService;
use crate::events::AppEvent;

pub struct AppState {
    pub db: PgPool,
    pub settings: SettingsRegistry,
    pub entitlements: Arc<EntitlementService>,

    pub events: Sender<AppEvent>,
    pub http: reqwest::Client,
    pub ai_provider_key: String,
    pub ai_api_endpoint: String,
    pub ai_model: String,
    /// Google Sheets API key for the destiny2 endgame-analysis sheet and
    /// compendium.
    pub google_api_key: String,
    pub error_log_webhook: String,
    pub normal_log_webhook: String,
    /// Discord user/application ID of the bot itself.
    pub zayden_id: u64,
    pub zayden_guild: u64,
}

impl AppState {
    #[must_use]
    pub fn new(pool: PgPool, config: &BotConfig) -> Self {
        let (events, _) = broadcast::channel(64);

        let settings = SettingsRegistry::new(pool.clone(), &events);

        let entitlements =
            Arc::new(EntitlementService::new(pool.clone(), events.clone()));
        EntitlementService::spawn_invalidator(
            Arc::clone(&entitlements),
            events.subscribe(),
        );

        Self {
            db: pool,
            settings,
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
            zayden_id: config.zayden_id,
            zayden_guild: config.zayden_guild,
        }
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.events.subscribe()
    }
}
