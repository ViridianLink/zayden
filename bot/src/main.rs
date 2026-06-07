use std::fs::File;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use serenity::all::{ClientBuilder, GatewayIntents, GuildId, Http, Token, UserId};
use sqlx::PgPool;
use tokio::sync::{OnceCell, RwLock};
use tracing::info;

pub mod bindings;
pub mod cron;
mod error;
mod handler;
pub mod registry;
pub mod sqlx_lib;
pub mod state;
pub mod webhook_logger;

pub use error::{BotError, Result};
pub use handler::Handler;
pub use registry::{CommandRegistry, RegistryBuilder};
pub use state::BotState;
use tracing::warn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, filter, fmt};
use zayden_app::events::listener::EventListener;

use crate::sqlx_lib::new_pool_with_retry;
use crate::webhook_logger::WebhookLogger;

pub const OSCAR_SIX: UserId = UserId::new(211_486_447_369_322_506);
pub const ZAYDEN_GUILD: GuildId = GuildId::new(1_222_360_995_700_150_443);
pub const LLAMAD2_GUILD: GuildId = GuildId::new(1_133_034_263_579_734_037);
pub const ZAYDEN_ID: UserId = UserId::new(787_490_197_943_091_211);

pub static ZAYDEN_TOKEN: OnceCell<String> = OnceCell::const_new();

async fn zayden_token(pool: &PgPool) -> sqlx::Result<String> {
    sqlx::query_scalar!("SELECT token FROM bot_tokens WHERE name = 'zayden'")
        .fetch_one(pool)
        .await
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(dotenvy::Error::Io(_)) = dotenvy::dotenv()
        && dotenvy::from_path(Path::new("bot/.env")).is_err()
    {
        warn!(".env file not found. Please make sure enviroment variables are set.");
    }

    let pool = new_pool_with_retry().await?;

    let bot_config = zayden_app::config::BotConfig::load(&pool).await?;
    info!("BotConfig loaded successfully");

    let app_state =
        Arc::new(zayden_app::state::AppState::new(pool.clone(), &bot_config));
    info!("AppState constructed successfully");

    EventListener::spawn(pool.clone(), app_state.events.clone());

    let bot_state =
        Arc::new(RwLock::new(BotState::new(Arc::clone(&app_state), &bot_config)?));

    let registry =
        bindings::build_registry().map_err(|e| BotError::Other(e.to_string()))?;

    let mut client = ClientBuilder::new(
        bot_config.discord_token.parse::<Token>().map_err(serenity::Error::Token)?,
        GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::GUILD_MESSAGE_REACTIONS
            | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_PRESENCES,
    )
    .data(Arc::clone(&bot_state))
    .event_handler(Arc::new(Handler {
        app: Arc::clone(&app_state),
        bot_state,
        registry,
        cron_started: OnceLock::new(),
    }))
    .await?;

    logging(
        Arc::clone(&client.http),
        bot_config.error_log_webhook.as_deref(),
        bot_config.normal_log_webhook.as_deref(),
    )
    .await;

    client.start_autosharded().await?;

    Ok(())
}

async fn logging(
    http: Arc<Http>,
    error_log_url: Option<&str>,
    normal_log_url: Option<&str>,
) {
    let debug_log = match File::create("log.txt") {
        Ok(log_file) => Some(
            fmt::layer()
                .with_writer(log_file)
                .with_filter(filter::LevelFilter::INFO),
        ),
        Err(e) => {
            eprintln!(
                "warning: failed to create log.txt, file logging disabled: {e}"
            );
            None
        },
    };

    let stdout_log = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(filter::LevelFilter::INFO);

    let webhook_log = WebhookLogger::new(http, error_log_url, normal_log_url).await;

    Registry::default().with(debug_log).with(stdout_log).with(webhook_log).init();
}
