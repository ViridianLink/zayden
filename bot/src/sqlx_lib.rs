use std::env;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub trait PostgresPool {
    async fn new_pool() -> sqlx::Result<PgPool> {
        PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .connect(&env::var("DATABASE_URL").unwrap())
            .await
    }

    fn pool(&self) -> &PgPool;
}

pub struct GuildTable;
