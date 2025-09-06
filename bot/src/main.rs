use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use endgame_analysis::endgame_analysis::EndgameAnalysisSheet;
use music::MusicData;
use serenity::all::{ClientBuilder, GatewayIntents, GuildId, Token, UserId};
use songbird::Songbird;
use sqlx::{PgPool, Postgres};
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
use modules::destiny2::endgame_analysis::DestinyWeaponTable;
use modules::destiny2::endgame_analysis::database_manager::DestinyDatabaseManager;
use sqlx_lib::new_pool;

pub const SUPER_USERS: [UserId; 1] = [
    UserId::new(211486447369322506), // oscarsix
];
pub const BRADSTER_GUILD: GuildId = GuildId::new(1255957182457974875);
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
    if let Err(dotenvy::Error::Io(_)) = dotenvy::dotenv() {
        dotenvy::from_path(Path::new("bot/.env")).unwrap()
    }

    // if dotenvy::from_path(Path::new("bot/.env")).is_err() {
    //     println!(".env file not found. Please make sure enviroment variables are set.")
    // }

    let pool = new_pool().await.unwrap();

    if !cfg!(debug_assertions) {
        DestinyDatabaseManager::update_dbs(&pool).await.unwrap();
        EndgameAnalysisSheet::update::<Postgres, DestinyWeaponTable>(&pool)
            .await
            .unwrap();
    }

    let data = CtxData::new(pool);

    let mut client = ClientBuilder::new(
        Token::from_env("DISCORD_TOKEN").unwrap(),
        GatewayIntents::all(),
    )
    .voice_manager::<Songbird>(data.songbird())
    .data(Arc::new(RwLock::new(data)))
    .event_handler(Handler {
        started_cron: AtomicBool::new(false),
    })
    .await
    .unwrap();

    client.start().await?;

    Ok(())
}
