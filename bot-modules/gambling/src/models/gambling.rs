use async_trait::async_trait;
use serenity::all::UserId;
use sqlx::{Database, Pool};

#[async_trait]
pub trait GamblingManager<Db: Database> {
    async fn coins(conn: &mut Db::Connection, id: UserId) -> sqlx::Result<i64>;

    async fn max_bet(conn: &mut Db::Connection, id: UserId) -> sqlx::Result<i64>;

    async fn bet(pool: &Pool<Db>, id: UserId, bet: i64) -> sqlx::Result<bool>;

    async fn add_coins(
        conn: &mut Db::Connection,
        id: UserId,
        amount: i64,
    ) -> sqlx::Result<Db::QueryResult>;

    async fn add_gems(
        conn: &mut Db::Connection,
        id: UserId,
        amount: i64,
    ) -> sqlx::Result<Db::QueryResult>;
}
