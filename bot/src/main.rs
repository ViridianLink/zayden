use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use destiny2_core::BungieClientData;
use endgame_analysis::endgame_analysis::EndgameAnalysisSheet;
use serenity::all::{ClientBuilder, GatewayIntents, GuildId, Token, UserId};
use sqlx::PgPool;
use tokio::sync::{OnceCell, RwLock};

mod cron;
pub mod ctx_data;
mod error;
mod handler;
pub mod modules;
mod sqlx_lib;

pub use ctx_data::CtxData;
pub use error::{Error, Result};
pub use handler::Handler;
use sqlx_lib::new_pool;
use tracing::warn;
use tracing_subscriber::{
    Layer, Registry, filter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

pub const OSCAR_SIX: UserId = UserId::new(211486447369322506);
pub const BRADSTER_GUILD: GuildId = GuildId::new(1255957182457974875);
pub const ZAYDEN_GUILD: GuildId = GuildId::new(1222360995700150443);
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
    logging();

    if let Err(dotenvy::Error::Io(_)) = dotenvy::dotenv()
        && dotenvy::from_path(Path::new("bot/.env")).is_err()
    {
        warn!(".env file not found. Please make sure enviroment variables are set.")
    }

    let pool = new_pool().await.unwrap();

    let data = CtxData::default();

    if !cfg!(debug_assertions) {
        let item_manifest = {
            let client = data.bungie_client();
            let manifest = client.destiny_manifest().await.unwrap();
            client
                .destiny_inventory_item_definition(&manifest, "en")
                .await
                .unwrap()
        };
        EndgameAnalysisSheet::update(&item_manifest).await.unwrap();
        destiny2::compendium::update().await;
    }

    let mut client = ClientBuilder::new(
        Token::from_env("DISCORD_TOKEN").unwrap(),
        GatewayIntents::all(),
    )
    .data(Arc::new(RwLock::new(data)))
    .event_handler(Handler {
        pool,
        started_cron: AtomicBool::new(false),
    })
    .await
    .unwrap();

    client.start_autosharded().await?;

    Ok(())
}

fn logging() {
    let log_file = File::create("log.txt").expect("Failed to create log.txt");
    let debug_log = fmt::layer()
        .with_writer(log_file)
        .with_filter(filter::LevelFilter::INFO);

    // A layer for logging to standard output
    let stdout_log = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(filter::LevelFilter::INFO);

    // let traceback_file = File::create("traceback.txt").expect("Failed to create traceback.txt");
    // let traceback_log = fmt::layer()
    //     .with_writer(traceback_file)
    //     .with_filter(filter::LevelFilter::TRACE);

    Registry::default()
        .with(debug_log)
        .with(stdout_log)
        // .with(traceback_log)
        .init();
}
