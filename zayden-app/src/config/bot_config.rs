use std::env;
use std::path::Path;

use serde::Deserialize;
use sqlx::PgPool;

use crate::{Error, Result};

// Defaults matching the current hardcoded constants in bot/src/main.rs.
const DEFAULT_OSCAR_SIX: u64 = 211_486_447_369_322_506;
const DEFAULT_ZAYDEN_GUILD: u64 = 1_222_360_995_700_150_443;
const DEFAULT_LLAMAD2_GUILD: u64 = 1_133_034_263_579_734_037;
const DEFAULT_ZAYDEN_ID: u64 = 787_490_197_943_091_211;

// AI provider defaults (OpenRouter free tier).
const DEFAULT_AI_ENDPOINT: &str = "https://openrouter.ai/api/v1";
const DEFAULT_AI_MODEL: &str = "google/gemma-4-31b-it:free";

// Dashboard defaults.
const DEFAULT_FRONTEND_URL: &str = "http://localhost:5173";
const DEFAULT_REDIRECT_URI: &str = "http://localhost:3000/auth/callback";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

/// Static + deployment-level configuration for the bot process.
///
/// Populated once at startup via `BotConfig::load`; thereafter immutable.
/// Merge order (lowest → highest priority):
///   1. Environment variables (required secrets)
///   2. `config.toml` (optional, deployment-specific overrides)
///   3. `bot_config` SQL row (optional, highest-priority runtime overrides)
#[derive(Debug, Clone)]
pub struct BotConfig {
    /// Discord bot token.
    pub discord_token: String,
    /// Bungie API key for Destiny 2 integration.
    pub bungie_api_key: String,
    /// AI provider API key (used for `OpenRouter` or any OpenAI-compatible API).
    pub ai_provider_key: String,
    /// Google Sheets API key for endgame analysis / destiny2 compendium.
    pub google_api_key: String,
    /// Discord `OAuth2` client secret for the web dashboard.
    pub discord_client_secret: String,

    /// Base URL of the AI provider endpoint (e.g. `https://openrouter.ai/api/v1`).
    pub ai_api_endpoint: String,
    /// Model identifier passed to the AI provider (e.g. `google/gemini-2.5-flash`).
    pub ai_model: String,

    /// Discord user ID of the bot owner (Oscar Six).
    pub oscar_six: u64,
    /// Guild ID of the primary Zayden server.
    pub zayden_guild: u64,
    /// Guild ID of the `LlamaD2` server.
    pub llamad2_guild: u64,
    /// Discord user/application ID of the Zayden bot itself.
    pub zayden_id: u64,

    /// Discord webhook URL for error-level log messages.
    pub error_log_webhook: Option<String>,
    /// Discord webhook URL for info/warn-level log messages.
    pub normal_log_webhook: Option<String>,

    /// Base URL of the Leptos/Svelte frontend (used for CORS and OAuth redirect
    /// errors).
    pub frontend_url: String,
    /// Full `OAuth2` redirect URI registered with Discord
    /// (e.g. `http://localhost:3000/auth/callback`).
    pub redirect_uri: String,
    /// Socket address the dashboard HTTP server binds to
    /// (e.g. `0.0.0.0:3000`).
    pub bind_addr: String,
    /// Discord bot invite URL. If absent the `/invite` route returns 404.
    pub invite_url: Option<String>,
}

impl BotConfig {
    /// Load the bot configuration by merging three sources.
    ///
    /// Fails fast if any required environment variable is absent.
    pub async fn load(pool: &PgPool) -> Result<Self> {
        // 1. Required env vars — fail early with a clear message.
        let discord_token = require_env("DISCORD_TOKEN")?;
        let bungie_api_key = require_env("BUNGIE_API_KEY")?;
        let ai_provider_key = require_env("AI_PROVIDER_KEY")?;
        let google_api_key = require_env("GOOGLE_API_KEY")?;
        let discord_client_secret = require_env("DISCORD_CLIENT_SECRET")?;

        // 2. config.toml (optional file, silently skipped if absent).
        let toml_cfg = load_toml_config()?;

        // 3. bot_config SQL row (optional, takes precedence over config.toml).
        let db = load_db_row(pool).await?;

        Ok(Self {
            discord_token,
            bungie_api_key,
            ai_provider_key,
            google_api_key,
            discord_client_secret,

            ai_api_endpoint: toml_cfg
                .ai
                .endpoint
                .unwrap_or_else(|| DEFAULT_AI_ENDPOINT.to_owned()),
            ai_model: toml_cfg
                .ai
                .model
                .unwrap_or_else(|| DEFAULT_AI_MODEL.to_owned()),

            oscar_six: toml_cfg.ids.oscar_six.unwrap_or(DEFAULT_OSCAR_SIX),
            zayden_guild: toml_cfg.ids.zayden_guild.unwrap_or(DEFAULT_ZAYDEN_GUILD),
            llamad2_guild: toml_cfg
                .ids
                .llamad2_guild
                .unwrap_or(DEFAULT_LLAMAD2_GUILD),
            zayden_id: toml_cfg.ids.zayden_id.unwrap_or(DEFAULT_ZAYDEN_ID),

            // DB row overrides config.toml; config.toml overrides None.
            error_log_webhook: db
                .as_ref()
                .and_then(|r| r.error_log_webhook.clone())
                .or(toml_cfg.webhooks.error_log),
            normal_log_webhook: db
                .as_ref()
                .and_then(|r| r.normal_log_webhook.clone())
                .or(toml_cfg.webhooks.normal_log),

            frontend_url: toml_cfg
                .dashboard
                .frontend_url
                .unwrap_or_else(|| DEFAULT_FRONTEND_URL.to_owned()),
            redirect_uri: toml_cfg
                .dashboard
                .redirect_uri
                .unwrap_or_else(|| DEFAULT_REDIRECT_URI.to_owned()),
            bind_addr: toml_cfg
                .dashboard
                .bind_addr
                .unwrap_or_else(|| DEFAULT_BIND_ADDR.to_owned()),
            invite_url: toml_cfg.dashboard.invite_url,
        })
    }
}

// --- helpers ---

fn require_env(var: &str) -> Result<String> {
    env::var(var).map_err(|_e| Error::MissingEnvVar(var.to_owned()))
}

/// Reads `config.toml` from the working directory or `bot/config.toml` as a
/// fallback; returns an empty/default config when neither file exists.
fn load_toml_config() -> Result<TomlConfig> {
    let path = if Path::new("config.toml").exists() {
        Path::new("config.toml")
    } else if Path::new("bot/config.toml").exists() {
        Path::new("bot/config.toml")
    } else {
        return Ok(TomlConfig::default());
    };

    let content = std::fs::read_to_string(path)?;
    let cfg: TomlConfig = toml::from_str(&content)?;
    Ok(cfg)
}

/// Fetches the single-row `bot_config` override table (may not exist yet —
/// uses dynamic query so no compile-time DB check is needed).
async fn load_db_row(pool: &PgPool) -> Result<Option<DbConfigRow>> {
    let row = sqlx::query_as::<_, DbConfigRow>(
        "SELECT error_log_webhook, normal_log_webhook FROM bot_config WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

// --- TOML deserialization types ---

#[derive(Debug, Default, Deserialize)]
struct TomlConfig {
    #[serde(default)]
    ai: TomlAi,
    #[serde(default)]
    ids: TomlIds,
    #[serde(default)]
    webhooks: TomlWebhooks,
    #[serde(default)]
    dashboard: TomlDashboard,
}

#[derive(Debug, Default, Deserialize)]
struct TomlAi {
    /// Base URL of the AI provider (default: `OpenRouter`).
    endpoint: Option<String>,
    /// Model identifier (default: `google/gemini-2.5-flash`).
    model: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlIds {
    oscar_six: Option<u64>,
    zayden_guild: Option<u64>,
    llamad2_guild: Option<u64>,
    zayden_id: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlWebhooks {
    error_log: Option<String>,
    normal_log: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlDashboard {
    frontend_url: Option<String>,
    redirect_uri: Option<String>,
    bind_addr: Option<String>,
    invite_url: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DbConfigRow {
    error_log_webhook: Option<String>,
    normal_log_webhook: Option<String>,
}
