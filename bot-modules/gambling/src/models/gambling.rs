use serenity::all::UserId;
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, PgPool};
use zayden_core::as_i64;

use super::{MaxBet, Prestige};

struct BetLimits {
    level: i32,
    prestige: i64,
}

impl Prestige for BetLimits {
    fn prestige(&self) -> i64 {
        self.prestige
    }
}

impl MaxBet for BetLimits {
    fn level(&self) -> i32 {
        self.level
    }
}

pub struct GamblingManager;

impl GamblingManager {
    pub async fn coins(conn: &mut PgConnection, id: UserId) -> sqlx::Result<i64> {
        sqlx::query_file_scalar!("sql/GamblingManager/coins.sql", as_i64(id.get()))
            .fetch_one(conn)
            .await
    }

    pub async fn max_bet(conn: &mut PgConnection, id: UserId) -> sqlx::Result<i64> {
        let limits = sqlx::query_as!(
            BetLimits,
            r#"
            SELECT
                COALESCE(l.level, 0) AS "level!: i32",
                COALESCE(m.prestige, 0) AS "prestige!: i64"
            FROM
                (SELECT $1::BIGINT AS user_id) u
            LEFT JOIN
                levels l ON l.user_id = u.user_id
            LEFT JOIN
                gambling_mine m ON m.user_id = u.user_id
            "#,
            as_i64(id.get())
        )
        .fetch_one(conn)
        .await?;

        Ok(limits.max_bet())
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
