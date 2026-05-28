mod commands;

pub use commands::{Levels, Rank, Xp};

pub fn register(builder: &mut crate::RegistryBuilder) {
    builder
        .add_command(Levels)
        .add_command(Rank)
        .add_command(Xp)
        .add_component(Levels);
}

use async_trait::async_trait;
use levels::{FullLevelRow, LeaderboardRow, LevelsManager, RankRow, XpRow};
use serenity::all::UserId;
use sqlx::{PgPool, Postgres, postgres::PgQueryResult};

pub struct LevelsTable;

#[async_trait]
impl LevelsManager<Postgres> for LevelsTable {
    async fn leaderboard(
        pool: &PgPool,
        users: &[i64],
        page: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page - 1) * 10;

        sqlx::query_as!(
            LeaderboardRow,
            "SELECT user_id, xp, level, message_count FROM levels WHERE user_id = ANY($1) ORDER BY level DESC, xp DESC LIMIT 10 OFFSET $2",
            users,
            offset
        )
        .fetch_all(pool)
        .await
    }

    async fn user_rank(
        pool: &PgPool,
        user_id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let id = user_id.into().get() as i64;

        sqlx::query_scalar!(
    "SELECT row_number FROM (SELECT user_id, ROW_NUMBER() OVER (ORDER BY level DESC, xp DESC) FROM levels) AS ranked WHERE user_id = $1",
    id
)
        .fetch_one(pool)
        .await
    }

    async fn rank_row(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<RankRow>> {
        let id = id.into();

        sqlx::query_as!(
            RankRow,
            "SELECT xp, level FROM levels WHERE user_id = $1",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn xp_row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<XpRow>> {
        let id = id.into();

        sqlx::query_as!(
            XpRow,
            "SELECT xp, level, total_xp FROM levels WHERE user_id = $1",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn full_row(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<FullLevelRow>> {
        let id = id.into();

        sqlx::query_as!(
            FullLevelRow,
            r#"SELECT user_id, xp, level, total_xp, message_count, last_xp as "last_xp: jiff_sqlx::Timestamp" FROM levels WHERE user_id = $1"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: FullLevelRow) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, 'PLACEHOLDER') ON CONFLICT (id) DO NOTHING",
            row.user_id
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "INSERT INTO levels (user_id, xp, total_xp, level, message_count, last_xp)
            VALUES ($1, $2, $3, $4, $5, now())
            ON CONFLICT (user_id) DO UPDATE
            SET xp = EXCLUDED.xp,
                total_xp = EXCLUDED.total_xp,
                level = EXCLUDED.level,
                message_count = EXCLUDED.message_count,
                last_xp = now();",
            row.user_id,
            row.xp,
            row.total_xp as i32,
            row.level,
            row.message_count as i32,
        )
        .execute(pool)
        .await
    }
}
