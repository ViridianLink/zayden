use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use sqlx::PgPool;
use tracing::warn;

use crate::config::radio;
use crate::config::radio::RadioStation;
use crate::{Error, Result};

const DEFAULT_OSCAR_SIX: u64 = 211_486_447_369_322_506;
const DEFAULT_ZAYDEN_GUILD: u64 = 1_222_360_995_700_150_443;
const DEFAULT_LLAMAD2_GUILD: u64 = 1_133_034_263_579_734_037;
const DEFAULT_ZAYDEN_ID: u64 = 787_490_197_943_091_211;

const DEFAULT_AI_ENDPOINT: &str = "https://openrouter.ai/api/v1";
const DEFAULT_AI_MODEL: &str = "openrouter/free";
const DEFAULT_AI_MODEL_STRUCTURED: &str = "nvidia/nemotron-3-super-120b-a12b:free";
const DEFAULT_AI_MODEL_PRO: &str = "google/gemini-2.5-flash";

const DEFAULT_REDIRECT_URI: &str = "http://localhost:3000/auth/callback";
const DEFAULT_PATREON_REDIRECT_URI: &str = "http://localhost:3000/patreon/callback";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

const DEFAULT_PALWORLD_SAVE_DIR: &str = "056C426C55974CFCA115EB695A224F67";
const DEFAULT_PALWORLD_UPLOADS_DIR: &str = "palworld_uploads";

#[derive(Debug, Clone)]
pub struct SpotifyCredentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone)]
pub struct PelicanConfig {
    pub base_url: String,
    pub api_key: String,
    pub save: Option<PelicanSaveConfig>,
}

#[derive(Debug, Clone)]
pub struct PelicanSaveConfig {
    pub server_id: String,
    pub save_path: String,
}

#[derive(Debug, Clone)]
pub struct HostingGame {
    pub key: String,
    pub name: String,
    pub egg_id: i32,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct HostingConfig {
    pub panel_url: String,
    pub kofi_url: String,
    pub location_ids: Vec<i32>,
    /// Hours, not days. The trial exists to prove the server comes up and the
    /// specs are what was advertised, not to be a free tier.
    pub trial_hours: i64,
    /// Days a *subscription* survives after lapsing. Never applied to a trial.
    pub grace_days: i64,
    pub reminder_days: i64,
    pub max_servers: i64,
    pub max_ram_mib: i64,
    pub games: Vec<HostingGame>,
}

#[derive(Debug, Clone)]
pub struct JellyfinConfig {
    pub internal_url: String,
    pub public_url: String,
    pub api_key: String,
    pub movie_library_id: String,
    pub show_library_id: String,
    pub seer_base_url: String,
    pub seer_api_key: String,
    pub region: String,
    pub dddie_api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PatreonConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone)]
pub struct BotConfig {
    pub discord_token: String,
    pub bungie_api_key: String,
    pub ai_provider_key: String,
    pub google_api_key: String,
    pub discord_client_secret: String,
    pub spotify: Option<SpotifyCredentials>,

    pub ai_api_endpoint: String,
    pub ai_model: String,
    pub ai_model_structured: String,
    pub ai_model_pro: String,

    pub bot_owner: u64,
    pub zayden_guild: u64,
    pub llamad2_guild: u64,
    /// Discord user/application ID of the Zayden bot itself.
    pub zayden_id: u64,

    pub error_log_webhook: Option<String>,
    pub normal_log_webhook: Option<String>,

    pub flaresolverr_url: Option<String>,

    pub youtube_cookies: Option<PathBuf>,

    pub palworld_paldex_url: Option<String>,

    pub palworld_palcalc_url: Option<String>,

    pub palworld_save_dir: Option<PathBuf>,
    pub palworld_uploads_dir: PathBuf,
    pub pelican: Option<PelicanConfig>,

    pub jellyfin: Option<JellyfinConfig>,

    pub hosting: Option<HostingConfig>,

    pub redirect_uri: String,
    pub bind_addr: String,
    pub invite_url: Option<String>,
    pub upgrade_url: Option<String>,

    pub kofi_verification_token: Option<String>,

    pub patreon: Option<PatreonConfig>,

    pub discord_sku_pro: Option<u64>,
    pub discord_sku_ultra: Option<u64>,

    pub radio_stations: Arc<[RadioStation]>,
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

        let pelican = load_pelican_config(&toml_cfg);
        let jellyfin = load_jellyfin_config(&toml_cfg);
        let hosting = load_hosting_config(&toml_cfg, pelican.as_ref());
        let patreon = load_patreon_config(
            toml_cfg
                .dashboard
                .patreon_redirect_uri
                .clone()
                .unwrap_or_else(|| DEFAULT_PATREON_REDIRECT_URI.to_owned()),
        );

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
            ai_model_structured: toml_cfg
                .ai
                .model_structured
                .unwrap_or_else(|| DEFAULT_AI_MODEL_STRUCTURED.to_owned()),
            ai_model_pro: toml_cfg
                .ai
                .model_pro
                .unwrap_or_else(|| DEFAULT_AI_MODEL_PRO.to_owned()),

            bot_owner: toml_cfg.ids.oscar_six.unwrap_or(DEFAULT_OSCAR_SIX),
            zayden_guild: toml_cfg.ids.zayden_guild.unwrap_or(DEFAULT_ZAYDEN_GUILD),
            llamad2_guild: toml_cfg
                .ids
                .llamad2_guild
                .unwrap_or(DEFAULT_LLAMAD2_GUILD),
            zayden_id: toml_cfg.ids.zayden_id.unwrap_or(DEFAULT_ZAYDEN_ID),

            error_log_webhook: db
                .as_ref()
                .and_then(|r| r.error_log_webhook.clone())
                .or_else(|| env::var("ERROR_LOG_WEBHOOK").ok()),
            normal_log_webhook: db
                .as_ref()
                .and_then(|r| r.normal_log_webhook.clone())
                .or_else(|| env::var("NORMAL_LOG_WEBHOOK").ok()),

            flaresolverr_url: env::var("FLARESOLVERR_URL").ok(),

            youtube_cookies: youtube_cookies_path(),

            palworld_save_dir: Some(
                toml_cfg.pelican.save_path.as_deref().map_or_else(
                    || PathBuf::from(DEFAULT_PALWORLD_SAVE_DIR),
                    save_dir_from_path,
                ),
            ),
            palworld_uploads_dir: toml_cfg.palworld.uploads_dir.map_or_else(
                || PathBuf::from(DEFAULT_PALWORLD_UPLOADS_DIR),
                PathBuf::from,
            ),
            pelican,

            jellyfin,

            hosting,

            palworld_paldex_url: toml_cfg.palworld.paldex_url,
            palworld_palcalc_url: toml_cfg.palworld.palcalc_url,

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
            kofi_verification_token: env::var("KOFI_VERIFICATION_TOKEN").ok(),

            patreon,

            discord_sku_pro: toml_cfg.entitlements.discord.skus.pro,
            discord_sku_ultra: toml_cfg.entitlements.discord.skus.ultra,

            radio_stations: radio::validate_all(radio::load()?),
        })
    }
}

fn youtube_cookies_path() -> Option<PathBuf> {
    let raw = env::var("YOUTUBE_COOKIES_FILE").ok()?;
    let path = PathBuf::from(raw.trim());

    if path.as_os_str().is_empty() {
        return None;
    }

    if path.is_file() {
        return Some(path);
    }

    warn!(
        "YOUTUBE_COOKIES_FILE points at {}, which is not a readable file; \
         YouTube playback will fall back to anonymous requests",
        path.display()
    );
    None
}

fn require_env(var: &str) -> Result<String> {
    env::var(var).map_err(|_e| Error::MissingEnvVar(var.to_owned()))
}

fn load_pelican_config(toml_cfg: &TomlConfig) -> Option<PelicanConfig> {
    let (base_url, api_key) = match (
        env::var("PELICAN_BASE_URL").ok(),
        env::var("PELICAN_API_KEY").ok(),
    ) {
        (Some(base_url), Some(api_key)) => (base_url, api_key),
        (None, None) => return None,
        _ => {
            warn!(
                "Pelican config is incomplete; both PELICAN_BASE_URL and \
                     PELICAN_API_KEY (env) must be set"
            );
            return None;
        },
    };

    let save = match (
        toml_cfg.pelican.server_id.clone(),
        toml_cfg.pelican.save_path.clone(),
    ) {
        (Some(server_id), Some(save_path)) => {
            Some(PelicanSaveConfig { server_id, save_path })
        },
        (None, None) => None,
        _ => {
            warn!(
                "Pelican save config is incomplete; near-live Palworld save \
                 refresh disabled until both [pelican].server_id and \
                 [pelican].save_path (config.toml) are set"
            );
            None
        },
    };

    Some(PelicanConfig { base_url, api_key, save })
}

const DEFAULT_HOSTING_TRIAL_HOURS: i64 = 4;
const DEFAULT_HOSTING_GRACE_DAYS: i64 = 3;
const DEFAULT_HOSTING_REMINDER_DAYS: i64 = 2;
const DEFAULT_HOSTING_MAX_SERVERS: i64 = 13;
const DEFAULT_HOSTING_MAX_RAM_MIB: i64 = 16_384;

fn load_hosting_config(
    toml_cfg: &TomlConfig,
    pelican: Option<&PelicanConfig>,
) -> Option<HostingConfig> {
    let cfg = &toml_cfg.hosting;

    if cfg.games.is_empty() && cfg.kofi_url.is_none() {
        return None;
    }

    let Some(pelican) = pelican else {
        warn!(
            "[hosting] is configured but Pelican credentials are not; game \
             server hosting disabled until PELICAN_BASE_URL and \
             PELICAN_API_KEY (env) are set"
        );
        return None;
    };

    let Some(kofi_url) = cfg.kofi_url.clone() else {
        warn!(
            "[hosting].kofi_url is unset; game server hosting disabled because \
             users would have no way to pay"
        );
        return None;
    };

    if cfg.games.is_empty() {
        warn!("[hosting] has no [[hosting.games]] entries; hosting disabled");
        return None;
    }

    // A zero or negative trial expires the server the moment it finishes
    // installing, which reads as a provisioning failure to whoever bought it.
    let trial_hours = match cfg.trial_hours {
        Some(hours) if hours > 0 => hours,
        Some(bad) => {
            warn!(
                configured = bad,
                default = DEFAULT_HOSTING_TRIAL_HOURS,
                "[hosting].trial_hours must be positive; using the default"
            );
            DEFAULT_HOSTING_TRIAL_HOURS
        },
        None => DEFAULT_HOSTING_TRIAL_HOURS,
    };

    let games = cfg
        .games
        .iter()
        .map(|g| HostingGame {
            key: g.key.clone(),
            name: g.name.clone(),
            egg_id: g.egg_id,
            env: g.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        })
        .collect();

    Some(HostingConfig {
        panel_url: cfg.panel_url.clone().unwrap_or_else(|| pelican.base_url.clone()),
        kofi_url,
        location_ids: cfg.location_ids.clone(),
        trial_hours,
        grace_days: cfg.grace_days.unwrap_or(DEFAULT_HOSTING_GRACE_DAYS),
        reminder_days: cfg.reminder_days.unwrap_or(DEFAULT_HOSTING_REMINDER_DAYS),
        max_servers: cfg.max_servers.unwrap_or(DEFAULT_HOSTING_MAX_SERVERS),
        max_ram_mib: cfg.max_ram_mib.unwrap_or(DEFAULT_HOSTING_MAX_RAM_MIB),
        games,
    })
}

const DEFAULT_JELLYFIN_REGION: &str = "GB";

fn load_jellyfin_config(toml_cfg: &TomlConfig) -> Option<JellyfinConfig> {
    let cfg = &toml_cfg.jellyfin;

    let endpoints = [
        cfg.public_url.clone(),
        cfg.movie_library_id.clone(),
        cfg.show_library_id.clone(),
        cfg.seer_base_url.clone(),
    ];
    let keys =
        [env::var("JELLYFIN_API_KEY").ok(), env::var("JELLYSEERR_API_KEY").ok()];

    if cfg.internal_url.is_none()
        && endpoints.iter().all(Option::is_none)
        && keys.iter().all(Option::is_none)
    {
        return None;
    }

    let (
        [
            Some(public_url),
            Some(movie_library_id),
            Some(show_library_id),
            Some(seer_base_url),
        ],
        [Some(api_key), Some(seer_api_key)],
    ) = (endpoints, keys)
    else {
        warn!(
            "Jellyfin config is incomplete; the /jellyfin and /watch commands \
             are disabled until [jellyfin].public_url, \
             [jellyfin].movie_library_id, [jellyfin].show_library_id and \
             [jellyfin].seer_base_url \
             (config.toml) plus JELLYFIN_API_KEY and JELLYSEERR_API_KEY (env) \
             are all set"
        );
        return None;
    };

    Some(JellyfinConfig {
        internal_url: cfg
            .internal_url
            .clone()
            .unwrap_or_else(|| public_url.clone()),
        public_url,
        api_key,
        movie_library_id,
        show_library_id,
        seer_base_url,
        seer_api_key,
        region: cfg
            .region
            .clone()
            .unwrap_or_else(|| DEFAULT_JELLYFIN_REGION.to_owned()),
        dddie_api_key: env::var("DOESTHEDOGDIE_API_KEY").ok(),
    })
}

fn load_patreon_config(redirect_uri: String) -> Option<PatreonConfig> {
    match (
        env::var("PATREON_CLIENT_ID").ok(),
        env::var("PATREON_CLIENT_SECRET").ok(),
    ) {
        (Some(client_id), Some(client_secret)) => {
            Some(PatreonConfig { client_id, client_secret, redirect_uri })
        },
        (None, None) => None,
        _ => {
            warn!(
                "Patreon config is incomplete; post announcements disabled until \
                 both PATREON_CLIENT_ID and PATREON_CLIENT_SECRET are set"
            );
            None
        },
    }
}

fn save_dir_from_path(save_path: &str) -> PathBuf {
    let name =
        save_path.trim_end_matches('/').rsplit('/').next().unwrap_or(save_path);
    PathBuf::from(name)
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
    let row = sqlx::query_as!(
        DbConfigRow,
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
    dashboard: TomlDashboard,
    #[serde(default)]
    palworld: TomlPalworld,
    #[serde(default)]
    pelican: TomlPelican,
    #[serde(default)]
    hosting: TomlHosting,
    #[serde(default)]
    jellyfin: TomlJellyfin,
    #[serde(default)]
    entitlements: TomlEntitlements,
}

#[derive(Debug, Default, Deserialize)]
struct TomlEntitlements {
    #[serde(default)]
    discord: TomlDiscordEntitlements,
}

#[derive(Debug, Default, Deserialize)]
struct TomlDiscordEntitlements {
    #[serde(default)]
    skus: TomlDiscordSkus,
}

#[derive(Debug, Default, Deserialize)]
struct TomlDiscordSkus {
    pro: Option<u64>,
    ultra: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlPalworld {
    paldex_url: Option<String>,
    palcalc_url: Option<String>,
    uploads_dir: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlJellyfin {
    internal_url: Option<String>,
    public_url: Option<String>,
    movie_library_id: Option<String>,
    show_library_id: Option<String>,
    seer_base_url: Option<String>,
    region: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlPelican {
    server_id: Option<String>,
    save_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlHosting {
    panel_url: Option<String>,
    kofi_url: Option<String>,
    #[serde(default)]
    location_ids: Vec<i32>,
    trial_hours: Option<i64>,
    grace_days: Option<i64>,
    reminder_days: Option<i64>,
    max_servers: Option<i64>,
    max_ram_mib: Option<i64>,
    #[serde(default)]
    games: Vec<TomlHostingGame>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlHostingGame {
    key: String,
    name: String,
    egg_id: i32,
    #[serde(default)]
    env: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlAi {
    endpoint: Option<String>,
    model: Option<String>,
    model_structured: Option<String>,
    model_pro: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlIds {
    oscar_six: Option<u64>,
    zayden_guild: Option<u64>,
    llamad2_guild: Option<u64>,
    zayden_id: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlDashboard {
    redirect_uri: Option<String>,
    patreon_redirect_uri: Option<String>,
    bind_addr: Option<String>,
    invite_url: Option<String>,
    upgrade_url: Option<String>,
}

struct DbConfigRow {
    error_log_webhook: Option<String>,
    normal_log_webhook: Option<String>,
}
