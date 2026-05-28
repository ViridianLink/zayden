use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use endgame_analysis::endgame_analysis::EndgameAnalysisSheet;
use serenity::all::{ClientBuilder, GatewayIntents, GuildId, Http, Token, UserId};
use sqlx::PgPool;
use tokio::sync::{OnceCell, RwLock};
use tracing::info;

mod cron;
pub mod ctx_data;
mod error;
mod handler;
pub mod modules;
mod sqlx_lib;
mod webhook_logger;

pub use ctx_data::CtxData;
pub use error::{Error, Result};
pub use handler::Handler;
use tracing::warn;
use tracing_subscriber::{
    Layer, Registry, filter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{sqlx_lib::new_pool_with_retry, webhook_logger::WebhookLogger};

pub const OSCAR_SIX: UserId = UserId::new(211486447369322506);
pub const BRADSTER_GUILD: GuildId = GuildId::new(1255957182457974875);
pub const ZAYDEN_GUILD: GuildId = GuildId::new(1222360995700150443);
pub const LLAMAD2_GUILD: GuildId = GuildId::new(1133034263579734037);
pub const ZAYDEN_ID: UserId = UserId::new(787490197943091211);

pub static ZAYDEN_TOKEN: OnceCell<String> = OnceCell::const_new();

async fn zayden_token(pool: &PgPool) -> String {
    sqlx::query_scalar!("SELECT token FROM bot_tokens WHERE name = 'zayden'")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(dotenvy::Error::Io(_)) = dotenvy::dotenv()
        && dotenvy::from_path(Path::new("bot/.env")).is_err()
    {
        warn!(".env file not found. Please make sure enviroment variables are set.")
    }

    let pool = new_pool_with_retry()
        .await
        .expect("Failed to connect to database.");

    let bot_config = zayden_app::config::BotConfig::load(&pool).await?;
    info!("BotConfig loaded successfully");

    let _app_state = zayden_app::state::AppState::new(pool.clone(), &bot_config);
    info!("AppState constructed successfully");

    let data = CtxData::default();

    if !cfg!(debug_assertions) {
        let manifest = EndgameAnalysisSheet::item_manifest(&data).await;
        EndgameAnalysisSheet::update(&manifest).await.unwrap();
        destiny2::compendium::update().await;
    }

    let mut client = ClientBuilder::new(
        Token::from_env("DISCORD_TOKEN").unwrap(),
        GatewayIntents::all(),
    )
    .data(Arc::new(RwLock::new(data)))
    .event_handler(Arc::new(Handler {
        pool,
        started_cron: AtomicBool::new(false),
    }))
    .await
    .unwrap();

    logging(Arc::clone(&client.http)).await;

    client.start_autosharded().await?;

    Ok(())
}

async fn logging(http: Arc<Http>) {
    let log_file = File::create("log.txt").expect("Failed to create log.txt");
    let debug_log = fmt::layer()
        .with_writer(log_file)
        .with_filter(filter::LevelFilter::INFO);

    // A layer for logging to standard output
    let stdout_log = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(filter::LevelFilter::INFO);

    let webhook_log = WebhookLogger::new(http).await;

    Registry::default()
        .with(debug_log)
        .with(stdout_log)
        .with(webhook_log)
        .init();
}
