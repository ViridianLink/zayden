#![expect(
    clippy::redundant_pub_crate,
    reason = "`redundant_pub_crate` and `unreachable_pub` are contradictory in \
              binary-crate submodules: every visibility that grants parent access \
              triggers one lint or the other."
)]

use std::fs::File;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use endgame_analysis::endgame_analysis::EndgameAnalysisSheet;
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

async fn zayden_token(pool: &PgPool) -> String {
    sqlx::query_scalar!("SELECT token FROM bot_tokens WHERE name = 'zayden'")
        .fetch_one(pool)
        .await
        .expect("failed to fetch zayden bot token from DB")
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(dotenvy::Error::Io(_)) = dotenvy::dotenv()
        && dotenvy::from_path(Path::new("bot/.env")).is_err()
    {
        warn!(".env file not found. Please make sure enviroment variables are set.");
    }

    let pool = new_pool_with_retry().await.expect("Failed to connect to database.");

    let bot_config = zayden_app::config::BotConfig::load(&pool).await?;
    info!("BotConfig loaded successfully");

    let app_state =
        Arc::new(zayden_app::state::AppState::new(pool.clone(), &bot_config));
    info!("AppState constructed successfully");

    EventListener::spawn(pool.clone(), app_state.events.clone());

    let bot_state = BotState::new(Arc::clone(&app_state), &bot_config);

    if !cfg!(debug_assertions) {
        let manifest = EndgameAnalysisSheet::item_manifest(&bot_state).await;
        EndgameAnalysisSheet::update(&manifest, &bot_config.google_api_key)
            .await
            .expect("EndgameAnalysisSheet update failed at startup");
        destiny2::compendium::update(&bot_config.google_api_key).await;
    }

    let bot_state = Arc::new(RwLock::new(bot_state));

    let registry = bindings::build_registry().expect(
        "overlapping prefix registered at startup — this is a programmer error",
    );

    let mut client = ClientBuilder::new(
        bot_config
            .discord_token
            .parse::<Token>()
            .expect("DISCORD_TOKEN in BotConfig is not a valid Discord token"),
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
    let log_file = File::create("log.txt").expect("Failed to create log.txt");
    let debug_log =
        fmt::layer().with_writer(log_file).with_filter(filter::LevelFilter::INFO);

    let stdout_log = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(filter::LevelFilter::INFO);

    let webhook_log = WebhookLogger::new(http, error_log_url, normal_log_url).await;

    Registry::default().with(debug_log).with(stdout_log).with(webhook_log).init();
}
