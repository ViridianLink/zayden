use async_trait::async_trait;
use serenity::all::UserId;
use sqlx::Database;

#[async_trait]
pub trait StatsManager<Db: Database> {
    async fn higherlower(
        conn: &mut Db::Connection,
        user_id: impl Into<UserId> + Send,
        score: i32,
    ) -> sqlx::Result<Db::QueryResult>;
}
