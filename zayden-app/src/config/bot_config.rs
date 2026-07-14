use std::env;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sqlx::PgPool;
use tracing::warn;

use crate::{Error, Result};

const DEFAULT_OSCAR_SIX: u64 = 211_486_447_369_322_506;
const DEFAULT_ZAYDEN_GUILD: u64 = 1_222_360_995_700_150_443;
const DEFAULT_LLAMAD2_GUILD: u64 = 1_133_034_263_579_734_037;
const DEFAULT_ZAYDEN_ID: u64 = 787_490_197_943_091_211;

const DEFAULT_AI_ENDPOINT: &str = "https://openrouter.ai/api/v1";
const DEFAULT_AI_MODEL: &str = "google/gemma-4-31b-it:free";

const DEFAULT_FRONTEND_URL: &str = "http://localhost:5173";
const DEFAULT_REDIRECT_URI: &str = "http://localhost:3000/auth/callback";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

const DEFAULT_PALWORLD_SAVE_DIR: &str = "056C426C55974CFCA115EB695A224F67";

#[derive(Debug, Clone)]
pub struct SpotifyCredentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone)]
pub struct BotConfig {
    pub discord_token: String,
    pub bungie_api_key: String,
    pub ai_provider_key: String,
    /// Google Sheets API key for endgame analysis / destiny2 compendium.
    pub google_api_key: String,
    pub discord_client_secret: String,
    pub spotify: Option<SpotifyCredentials>,

    pub ai_api_endpoint: String,
    pub ai_model: String,

    pub bot_owner: u64,
    pub zayden_guild: u64,
    pub llamad2_guild: u64,
    /// Discord user/application ID of the Zayden bot itself.
    pub zayden_id: u64,

    pub error_log_webhook: Option<String>,
    pub normal_log_webhook: Option<String>,

    pub flaresolverr_url: Option<String>,

    pub palworld_paldex_url: Option<String>,

    pub palworld_palcalc_url: Option<String>,

    pub palworld_save_dir: Option<PathBuf>,

    pub frontend_url: String,
    pub redirect_uri: String,
    pub bind_addr: String,
    pub invite_url: Option<String>,
    pub upgrade_url: Option<String>,
}

impl BotConfig {
    pub async fn load(pool: &PgPool) -> Result<Self> {
        let discord_token = require_env("DISCORD_TOKEN")?;
        let bungie_api_key = require_env("BUNGIE_API_KEY")?;
        let ai_provider_key = require_env("AI_PROVIDER_KEY")?;
        let google_api_key = require_env("GOOGLE_API_KEY")?;
        let discord_client_secret = require_env("DISCORD_CLIENT_SECRET")?;
        let spotify = match (
            env::var("SPOTIFY_CLIENT_ID"),
            env::var("SPOTIFY_CLIENT_SECRET"),
        ) {
            (Ok(client_id), Ok(client_secret)) => {
                Some(SpotifyCredentials { client_id, client_secret })
            },
            (Err(_), Err(_)) => None,
            (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
                warn!(
                    "only one of SPOTIFY_CLIENT_ID/SPOTIFY_CLIENT_SECRET is set; \
                     Spotify support disabled until both are provided"
                );
                None
            },
        };

        let toml_cfg = load_toml_config()?;

        let db = load_db_row(pool).await?;

        Ok(Self {
            discord_token,
            bungie_api_key,
            ai_provider_key,
            google_api_key,
            discord_client_secret,
            spotify,

            ai_api_endpoint: toml_cfg
                .ai
                .endpoint
                .unwrap_or_else(|| DEFAULT_AI_ENDPOINT.to_owned()),
            ai_model: toml_cfg
                .ai
                .model
                .unwrap_or_else(|| DEFAULT_AI_MODEL.to_owned()),

            bot_owner: toml_cfg.ids.oscar_six.unwrap_or(DEFAULT_OSCAR_SIX),
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

            flaresolverr_url: env::var("FLARESOLVERR_URL").ok(),

            palworld_paldex_url: env::var("PALWORLD_PALDEX_URL").ok(),

            palworld_palcalc_url: env::var("PALWORLD_PALCALC_URL").ok(),

            palworld_save_dir: Some(env::var("PALWORLD_SAVE_DIR").map_or_else(
                |_| PathBuf::from(DEFAULT_PALWORLD_SAVE_DIR),
                PathBuf::from,
            )),

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
            upgrade_url: toml_cfg.dashboard.upgrade_url,
        })
    }
}

fn require_env(var: &str) -> Result<String> {
    env::var(var).map_err(|_e| Error::MissingEnvVar(var.to_owned()))
}

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

async fn load_db_row(pool: &PgPool) -> Result<Option<DbConfigRow>> {
    let row = sqlx::query_as::<_, DbConfigRow>(
        "SELECT error_log_webhook, normal_log_webhook FROM bot_config WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

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
    endpoint: Option<String>,
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
    upgrade_url: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DbConfigRow {
    error_log_webhook: Option<String>,
    normal_log_webhook: Option<String>,
}
