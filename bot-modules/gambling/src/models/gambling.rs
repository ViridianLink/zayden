use serenity::all::UserId;
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, PgPool};
use zayden_core::as_i64;

pub struct GamblingManager;

impl GamblingManager {
    pub async fn coins(conn: &mut PgConnection, id: UserId) -> sqlx::Result<i64> {
        sqlx::query_file_scalar!("sql/GamblingManager/coins.sql", as_i64(id.get()))
            .fetch_one(conn)
            .await
    }

    pub async fn max_bet(conn: &mut PgConnection, id: UserId) -> sqlx::Result<i64> {
        sqlx::query_scalar!(
            r#"
            SELECT
                (
                    GREATEST(l.level * 10000, 10000)
                    * (COALESCE(m.prestige, 0) + 10)
                ) / 10
            FROM
                levels l
            LEFT JOIN
                gambling_mine m ON l.user_id = m.user_id
            WHERE
                l.user_id = $1
            "#,
            as_i64(id.get())
        )
        .fetch_one(conn)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
    }

    // region: Update
    pub async fn bet(pool: &PgPool, id: UserId, bet: i64) -> sqlx::Result<bool> {
        let result =
            sqlx::query_file!("sql/GamblingManager/bet.sql", as_i64(id.get()), bet)
                .execute(pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn add_coins(
        conn: &mut PgConnection,
        id: UserId,
        amount: i64,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "sql/GamblingManager/add_coins.sql",
            as_i64(id.get()),
            amount
        )
        .execute(conn)
        .await
    }

    pub async fn add_gems(
        conn: &mut PgConnection,
        id: UserId,
        amount: i64,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "sql/GamblingManager/add_gems.sql",
            as_i64(id.get()),
            amount
        )
        .execute(conn)
        .await
    }
}
