use std::fs::File;
use std::path::Path;
use std::sync::Arc;

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

    {
        let events_tx = app_state.events.clone();
        let listener_pool = pool.clone();
        tokio::spawn(async move {
            zayden_app::events::listener::ConfigListener::listen(
                &listener_pool,
                events_tx,
            )
            .await;
        });
    }

    let bot_state = BotState::new(Arc::clone(&app_state), &bot_config);

    if !cfg!(debug_assertions) {
        let manifest = EndgameAnalysisSheet::item_manifest(&bot_state).await;
        EndgameAnalysisSheet::update(&manifest)
            .await
            .expect("EndgameAnalysisSheet update failed at startup");
        destiny2::compendium::update().await;
    }

    let bot_state = Arc::new(RwLock::new(bot_state));

    let registry = bindings::build_registry().expect(
        "overlapping prefix registered at startup — this is a programmer error",
    );

    let mut client = ClientBuilder::new(
        Token::from_env("DISCORD_TOKEN").expect("DISCORD_TOKEN env var must be set"),
        GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::GUILD_MESSAGE_REACTIONS
            | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_PRESENCES,
    )
    .data(Arc::clone(&bot_state))
    .event_handler(Arc::new(Handler { bot_state, registry }))
    .await?;

    logging(Arc::clone(&client.http)).await;

    client.start_autosharded().await?;

    Ok(())
}

async fn logging(http: Arc<Http>) {
    let log_file = File::create("log.txt").expect("Failed to create log.txt");
    let debug_log =
        fmt::layer().with_writer(log_file).with_filter(filter::LevelFilter::INFO);

    let stdout_log = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(filter::LevelFilter::INFO);

    let webhook_log = WebhookLogger::new(http).await;

    Registry::default().with(debug_log).with(stdout_log).with(webhook_log).init();
}
