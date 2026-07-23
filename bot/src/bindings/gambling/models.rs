use async_trait::async_trait;
use gambling::{GamblingManager, GameManager, GameRow, StatsManager};
use serenity::all::UserId;
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, PgPool, Postgres};
use zayden_core::as_i64;

pub struct GamblingTable;

#[async_trait]
impl GamblingManager<Postgres> for GamblingTable {
    async fn coins(conn: &mut PgConnection, id: UserId) -> sqlx::Result<i64> {
        sqlx::query_file_scalar!(
            "./sql/gambling/GamblingManager/coins.sql",
            as_i64(id.get())
        )
        .fetch_one(conn)
        .await
    }

    async fn max_bet(conn: &mut PgConnection, id: UserId) -> sqlx::Result<i64> {
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
    async fn bet(pool: &PgPool, id: UserId, bet: i64) -> sqlx::Result<bool> {
        let result = sqlx::query_file!(
            "./sql/gambling/GamblingManager/bet.sql",
            as_i64(id.get()),
            bet
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn add_coins(
        conn: &mut PgConnection,
        id: UserId,
        amount: i64,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "./sql/gambling/GamblingManager/add_coins.sql",
            as_i64(id.get()),
            amount
        )
        .execute(conn)
        .await
    }

    async fn add_gems(
        conn: &mut PgConnection,
        id: UserId,
        amount: i64,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "./sql/gambling/GamblingManager/add_gems.sql",
            as_i64(id.get()),
            amount
        )
        .execute(conn)
        .await
    }
}

pub struct GameTable;

#[async_trait]
impl GameManager<Postgres> for GameTable {
    async fn row(pool: &PgPool, id: UserId) -> sqlx::Result<Option<GameRow>> {
        sqlx::query_file_as!(
            GameRow,
            "./sql/gambling/GameManager/row.sql",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: GameRow) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO gambling (user_id, coins, gems)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems;",
            row.user_id,
            row.coins,
            row.gems,
        )
        .execute(pool)
        .await
    }
}

pub struct StatsTable;

#[async_trait]
impl StatsManager<Postgres> for StatsTable {
    async fn higherlower(
        conn: &mut PgConnection,
        user_id: UserId,
        score: i32,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "sql/gambling/StatsManager/higherlower.sql",
            as_i64(user_id.get()),
            score
        )
        .execute(conn)
        .await
    }
}
