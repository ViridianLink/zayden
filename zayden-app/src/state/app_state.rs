use std::collections::HashMap;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;

use crate::config::{BotConfig, RadioStation, SettingsRegistry};
use crate::entitlement::{EntitlementService, Tier};
use crate::events::AppEvent;
use crate::services::http::ClientBuilderExt;

fn http_client() -> reqwest::Client {
    reqwest::Client::builder().with_timeouts().build().unwrap_or_else(|e| {
        tracing::warn!(
            error = %e,
            "failed to build the shared HTTP client; falling back to the \
             default (no timeout)"
        );
        reqwest::Client::new()
    })
}

pub struct AppState {
    pub db: PgPool,
    pub settings: SettingsRegistry,
    pub entitlements: Arc<EntitlementService>,

    pub events: Sender<AppEvent>,
    pub http: reqwest::Client,
    pub discord_token: String,
    pub ai_provider_key: String,
    pub ai_api_endpoint: String,
    pub ai_model: String,
    pub ai_model_pro: String,
    pub google_api_key: String,
    pub error_log_webhook: String,
    pub normal_log_webhook: String,
    /// Discord user/application ID of the bot itself.
    pub zayden_id: u64,
    pub zayden_guild: u64,
    pub upgrade_url: Option<String>,

    pub sku_tiers: HashMap<u64, Tier>,
    pub radio_stations: Arc<[RadioStation]>,
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

        let mut sku_tiers = HashMap::new();
        if let Some(sku) = config.discord_sku_pro {
            sku_tiers.insert(sku, Tier::Pro);
        }
        if let Some(sku) = config.discord_sku_ultra {
            sku_tiers.insert(sku, Tier::Ultra);
        }

        Self {
            db: pool,
            settings,
            entitlements,
            events,
            http: http_client(),
            discord_token: config.discord_token.clone(),
            ai_provider_key: config.ai_provider_key.clone(),
            ai_api_endpoint: config.ai_api_endpoint.clone(),
            ai_model: config.ai_model.clone(),
            ai_model_pro: config.ai_model_pro.clone(),
            google_api_key: config.google_api_key.clone(),
            error_log_webhook: config.error_log_webhook.clone().unwrap_or_default(),
            normal_log_webhook: config
                .normal_log_webhook
                .clone()
                .unwrap_or_default(),
            zayden_id: config.zayden_id,
            zayden_guild: config.zayden_guild,
            upgrade_url: config.upgrade_url.clone(),
            sku_tiers,
            radio_stations: Arc::clone(&config.radio_stations),
        }
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.events.subscribe()
    }
}
