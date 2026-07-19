use async_trait::async_trait;
use jiff_sqlx::{Timestamp, ToSqlx};
use serenity::all::UserId;
use sqlx::{Database, FromRow, Pool};

use crate::GoldStarError;

#[async_trait]
pub trait GoldStarManager<Db: Database> {
    async fn get_row(
        pool: &Pool<Db>,
        user_id: impl Into<i64> + Send,
    ) -> sqlx::Result<Option<GoldStarRow>>;

    async fn give_star(
        pool: &Pool<Db>,
        author_id: UserId,
        target_id: UserId,
    ) -> Result<i32, GoldStarError>;
}

#[derive(FromRow)]
pub struct GoldStarRow {
    pub id: i64,
    pub number_of_stars: i32,
    pub given_stars: i32,
    pub received_stars: i32,
    pub last_free_star: Timestamp,
}

impl GoldStarRow {
    pub fn new(user_id: impl Into<i64>) -> Self {
        Self {
            id: user_id.into(),
            number_of_stars: 0,
            given_stars: 0,
            received_stars: 0,
            last_free_star: jiff::Timestamp::default().to_sqlx(),
        }
    }
}
