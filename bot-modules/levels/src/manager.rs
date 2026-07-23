use jiff_sqlx::{Timestamp, ToSqlx};
use serenity::all::UserId;
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use sqlx::prelude::FromRow;
use zayden_core::{as_i64, as_u64};

use crate::level_up_xp;

pub trait LevelsRow {
    fn user_id(&self) -> UserId;

    fn xp(&self) -> i32;

    fn level(&self) -> i32;

    fn total_xp(&self) -> i64;

    fn message_count(&self) -> i64;

    fn last_xp(&self) -> jiff::Timestamp;
}

#[derive(FromRow)]
pub struct LeaderboardRow {
    pub user_id: i64,
    pub xp: i32,
    pub level: i32,
    pub message_count: i64,
}

impl LeaderboardRow {
    pub async fn leaderboard(
        pool: &PgPool,
        users: &[i64],
        page: i64,
    ) -> sqlx::Result<Vec<Self>> {
        let offset = (page - 1) * 10;

        sqlx::query_as!(
            Self,
            "SELECT user_id, xp, level, message_count FROM levels WHERE user_id = ANY($1) ORDER BY level DESC, xp DESC LIMIT 10 OFFSET $2",
            users,
            offset
        )
        .fetch_all(pool)
        .await
    }
}

impl LevelsRow for LeaderboardRow {
    fn user_id(&self) -> UserId {
        UserId::new(as_u64(self.user_id))
    }

    fn xp(&self) -> i32 {
        self.xp
    }

    fn level(&self) -> i32 {
        self.level
    }

    fn total_xp(&self) -> i64 {
        0
    }

    fn message_count(&self) -> i64 {
        self.message_count
    }

    fn last_xp(&self) -> jiff::Timestamp {
        jiff::Timestamp::UNIX_EPOCH
    }
}

#[derive(FromRow)]
pub struct RankRow {
    pub xp: i32,
    pub level: i32,
}

impl RankRow {
    pub async fn get(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<Self>> {
        let id = id.into();

        sqlx::query_as!(
            Self,
            "SELECT xp, level FROM levels WHERE user_id = $1",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn user_rank(
        pool: &PgPool,
        user_id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let id = as_i64(user_id.into().get());

        sqlx::query_scalar!(
    "SELECT row_number FROM (SELECT user_id, ROW_NUMBER() OVER (ORDER BY level DESC, xp DESC) FROM levels) AS ranked WHERE user_id = $1",
    id
)
        .fetch_one(pool)
        .await
    }
}

impl Default for RankRow {
    fn default() -> Self {
        Self { xp: 0, level: 1 }
    }
}

impl LevelsRow for RankRow {
    fn user_id(&self) -> UserId {
        UserId::default()
    }

    fn xp(&self) -> i32 {
        self.xp
    }

    fn level(&self) -> i32 {
        self.level
    }

    fn total_xp(&self) -> i64 {
        0
    }

    fn message_count(&self) -> i64 {
        0
    }

    fn last_xp(&self) -> jiff::Timestamp {
        jiff::Timestamp::UNIX_EPOCH
    }
}

#[derive(FromRow)]
pub struct XpRow {
    pub xp: i32,
    pub level: i32,
    pub total_xp: i64,
}

impl XpRow {
    pub async fn get(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<Self>> {
        let id = id.into();

        sqlx::query_as!(
            Self,
            "SELECT xp, level, total_xp FROM levels WHERE user_id = $1",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }
}

impl Default for XpRow {
    fn default() -> Self {
        Self { xp: 0, level: 1, total_xp: 0 }
    }
}

impl LevelsRow for XpRow {
    fn user_id(&self) -> UserId {
        UserId::default()
    }

    fn xp(&self) -> i32 {
        self.xp
    }

    fn level(&self) -> i32 {
        self.level
    }

    fn total_xp(&self) -> i64 {
        self.total_xp
    }

    fn message_count(&self) -> i64 {
        0
    }

    fn last_xp(&self) -> jiff::Timestamp {
        jiff::Timestamp::UNIX_EPOCH
    }
}

#[derive(FromRow)]
pub struct FullLevelRow {
    pub user_id: i64,
    pub xp: i32,
    pub level: i32,
    pub total_xp: i64,
    pub message_count: i64,
    pub last_xp: Timestamp,
}

impl FullLevelRow {
    pub fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            user_id: as_i64(id.get()),
            xp: 0,
            level: 0,
            total_xp: 0,
            message_count: 0,
            last_xp: jiff::Timestamp::default().to_sqlx(),
        }
    }

    pub fn new_message(&mut self) -> Option<i32> {
        self.message_count += 1;

        let rand_xp = rand::random_range(15..25);
        self.total_xp += i64::from(rand_xp);
        self.xp += rand_xp;

        let next_level_xp = level_up_xp(self.level());
        if self.xp >= next_level_xp {
            self.xp -= next_level_xp;
            self.level += 1;
            return Some(self.level);
        }

        None
    }

    pub async fn get(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<Self>> {
        let id = id.into();

        sqlx::query_as!(
            Self,
            r#"SELECT user_id, xp, level, total_xp, message_count, last_xp as "last_xp: jiff_sqlx::Timestamp" FROM levels WHERE user_id = $1"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "DB columns xp/message_count are INT4; values are bounded by gameplay"
    )]
    pub async fn save(self, pool: &PgPool) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, 'PLACEHOLDER') ON CONFLICT (id) DO NOTHING",
            self.user_id
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
            self.user_id,
            self.xp,
            self.total_xp as i32,
            self.level,
            self.message_count as i32,
        )
        .execute(pool)
        .await
    }
}

impl LevelsRow for FullLevelRow {
    fn user_id(&self) -> UserId {
        UserId::new(as_u64(self.user_id))
    }

    fn xp(&self) -> i32 {
        self.xp
    }

    fn level(&self) -> i32 {
        self.level
    }

    fn total_xp(&self) -> i64 {
        self.total_xp
    }

    fn message_count(&self) -> i64 {
        self.message_count
    }

    fn last_xp(&self) -> jiff::Timestamp {
        self.last_xp.to_jiff()
    }
}
