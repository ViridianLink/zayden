use async_trait::async_trait;
use gambling::{GamblingManager, GameManager, GameRow, StatsManager};
use serenity::all::UserId;
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, PgPool, Postgres};

pub struct GamblingTable;

#[async_trait]
impl GamblingManager<Postgres> for GamblingTable {
    async fn coins(
        conn: &mut PgConnection,
        id: impl Into<UserId> + std::marker::Send,
    ) -> sqlx::Result<i64> {
        let id = id.into();

        sqlx::query_file_scalar!("./sql/gambling/GamblingManager/coins.sql", id.get() as i64)
            .fetch_one(conn)
            .await
    }

    async fn max_bet(
        conn: &mut PgConnection,
        id: impl Into<UserId> + std::marker::Send,
    ) -> sqlx::Result<i64> {
        let id = id.into();

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
                gambling_mine m ON l.id = m.id
            WHERE
                l.id = $1
            "#,
            id.get() as i64
        )
        .fetch_one(conn)
        .await
        .map(|r| r.unwrap())
    }

    //region: Update
    async fn bet(
        pool: &PgPool,
        id: impl Into<UserId> + std::marker::Send,
        bet: i64,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();

        sqlx::query_file!(
            "./sql/gambling/GamblingManager/bet.sql",
            id.get() as i64,
            bet
        )
        .execute(pool)
        .await
    }

    async fn add_coins(
        conn: &mut PgConnection,
        id: impl Into<UserId> + std::marker::Send,
        amount: i64,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();

        sqlx::query_file!(
            "./sql/gambling/GamblingManager/add_coins.sql",
            id.get() as i64,
            amount
        )
        .execute(conn)
        .await
    }

    async fn add_gems(
        conn: &mut PgConnection,
        id: impl Into<UserId> + Send,
        amount: i64,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();

        sqlx::query_file!(
            "./sql/gambling/GamblingManager/add_gems.sql",
            id.get() as i64,
            amount
        )
        .execute(conn)
        .await
    }
}

pub struct GameTable;

#[async_trait]
impl GameManager<Postgres> for GameTable {
    async fn row(
        pool: &PgPool,
        id: impl Into<UserId> + std::marker::Send,
    ) -> sqlx::Result<Option<GameRow>> {
        let id = id.into();

        sqlx::query_file_as!(
            GameRow,
            "./sql/gambling/GameManager/row.sql",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: GameRow) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO gambling (id, coins, gems)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems;",
            row.id,
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
        user_id: impl Into<UserId> + Send,
        score: i32,
    ) -> sqlx::Result<PgQueryResult> {
        let user_id = user_id.into();

        sqlx::query_file!(
            "sql/gambling/StatsManager/higherlower.sql",
            user_id.get() as i64,
            score
        )
        .execute(conn)
        .await
    }
}
