use std::env;
use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::time::sleep;
use tracing::{error, info};

const MAX_LIFETIME: Duration = Duration::from_mins(30);

pub struct GuildTable;

pub async fn new_pool(url: &str) -> sqlx::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .max_lifetime(MAX_LIFETIME)
        .connect(url)
        .await
}

pub async fn new_pool_with_retry() -> sqlx::Result<PgPool> {
    let db_url = env::var("DATABASE_URL")
        .map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;

    let max_retries = 5;
    let retry_delay = Duration::from_secs(5);

    let mut attempts = 0;

    loop {
        attempts += 1;

        let result = new_pool(&db_url).await;

        match result {
            Err(sqlx::Error::Io(e)) => {
                if attempts >= max_retries {
                    error!("Failed to connect after {attempts} attempts.");
                    return Err(sqlx::Error::Io(e));
                }

                info!(
                    "Connection attempt {attempts}/{max_retries} failed: {e}. Retrying in {retry_delay:?}..."
                );

                sleep(retry_delay).await;
            },

            r => {
                return r;
            },
        }
    }
}
