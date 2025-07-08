use std::env;

use endgame_analysis::endgame_analysis::EndgameAnalysisSheet;
use gambling::{Lotto, StaminaCron};
use modules::destiny2::endgame_analysis::DestinyWeaponTable;
use modules::destiny2::endgame_analysis::database_manager::DestinyDatabaseManager;
use serenity::all::{ClientBuilder, GatewayIntents, GuildId, UserId};
use serenity::prelude::TypeMap;

pub use error::{Error, Result};
use sqlx::Postgres;
use sqlx_lib::PostgresPool;
use zayden_core::CronJobs;

use crate::modules::gambling::{GamblingTable, LottoTable, StaminaTable};

mod cron;
mod error;
mod handler;
pub mod modules;
mod sqlx_lib;

pub const SUPER_USERS: [UserId; 1] = [
    UserId::new(211486447369322506), // oscarsix
];
pub const BRADSTER_GUILD: GuildId = GuildId::new(1255957182457974875);

#[tokio::main]
async fn main() -> Result<()> {
    if dotenvy::dotenv().is_err() {
        println!(".env file not found. Please make sure enviroment variables are set.")
    }
    // if cfg!(debug_assertions) {
    //     let _ = dotenvy::from_filename_override(".env.dev");
    // }

    let pool = PostgresPool::new().await.unwrap();

    if !cfg!(debug_assertions) {
        DestinyDatabaseManager::update_dbs(&pool.pool)
            .await
            .unwrap();
        EndgameAnalysisSheet::update::<Postgres, DestinyWeaponTable>(&pool.pool)
            .await
            .unwrap();
    }

    let mut type_map = TypeMap::new();
    type_map.insert::<PostgresPool>(pool);
    type_map.insert::<CronJobs<Postgres>>(vec![
        Lotto::cron_job::<Postgres, GamblingTable, LottoTable>(),
        StaminaCron::cron_job::<Postgres, StaminaTable>(),
    ]);

    let token = &env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");

    let mut client = ClientBuilder::new(token, GatewayIntents::all())
        .type_map(type_map)
        .raw_event_handler(handler::Handler)
        .await
        .unwrap();

    client.start().await.unwrap();

    Ok(())
}
