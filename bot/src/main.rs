use std::sync::Arc;

use endgame_analysis::endgame_analysis::EndgameAnalysisSheet;
use serenity::all::{ClientBuilder, GatewayIntents, GuildId, Token, UserId};
use sqlx::Postgres;
use tokio::sync::RwLock;

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
use sqlx_lib::PostgresPool;

pub const SUPER_USERS: [UserId; 1] = [
    UserId::new(211486447369322506), // oscarsix
];
pub const BRADSTER_GUILD: GuildId = GuildId::new(1255957182457974875);
pub const BOT_ID: UserId = UserId::new(787490197943091211);

#[tokio::main]
async fn main() -> Result<()> {
    if dotenvy::dotenv().is_err() {
        println!(".env file not found. Please make sure enviroment variables are set.")
    }
    // if cfg!(debug_assertions) {
    //     let _ = dotenvy::from_filename_override(".env.dev");
    // }

    let data = CtxData::new().await?;

    if !cfg!(debug_assertions) {
        DestinyDatabaseManager::update_dbs(data.pool())
            .await
            .unwrap();
        EndgameAnalysisSheet::update::<Postgres, DestinyWeaponTable>(data.pool())
            .await
            .unwrap();
    }

    let mut client = ClientBuilder::new(
        Token::from_env("DISCORD_TOKEN").unwrap(),
        GatewayIntents::all(),
    )
    .voice_manager::<Songbird>(data.songbird())
    .data(Arc::new(RwLock::new(data)))
    .event_handler(handler::Handler)
    .await
    .unwrap();

    client.start().await.unwrap();

    Ok(())
}
