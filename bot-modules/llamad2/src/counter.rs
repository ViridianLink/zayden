use sqlx::PgPool;

use crate::Result;

pub struct Counter;

impl Counter {
    pub const COUNTING_FAILS: &str = "counting_fails";
    pub const DUMB_COUNT: &str = "dumb_count";

    pub async fn bump(pool: &PgPool, name: &str) -> Result<i64> {
        let count = sqlx::query_scalar!(
            "INSERT INTO llamad2_counters (name, count)
                 VALUES ($1, 1)
             ON CONFLICT (name)
                 DO UPDATE SET count = llamad2_counters.count + 1
             RETURNING count",
            name,
        )
        .fetch_one(pool)
        .await?;

        Ok(count)
    }
}
