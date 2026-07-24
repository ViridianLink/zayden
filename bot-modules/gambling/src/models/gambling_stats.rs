use serenity::all::UserId;
use sqlx::PgConnection;
use sqlx::postgres::PgQueryResult;
use zayden_core::as_i64;

pub struct StatsManager;

impl StatsManager {
    pub async fn higherlower(
        conn: &mut PgConnection,
        user_id: UserId,
        score: i32,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "sql/StatsManager/higherlower.sql",
            as_i64(user_id.get()),
            score
        )
        .execute(conn)
        .await
    }
}
