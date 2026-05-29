use async_trait::async_trait;
use jiff_sqlx::{Timestamp, ToSqlx};
use serenity::all::UserId;
use sqlx::{prelude::FromRow, Database, Pool};

use crate::level_up_xp;

#[async_trait]
pub trait LevelsManager<Db: Database> {
    async fn leaderboard(
        pool: &Pool<Db>,
        users: &[i64],
        page: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn user_rank(
        pool: &Pool<Db>,
        user_id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn rank_row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<RankRow>>;

    async fn xp_row(pool: &Pool<Db>, id: impl Into<UserId> + Send) -> sqlx::Result<Option<XpRow>>;

    async fn full_row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<FullLevelRow>>;

    async fn save(pool: &Pool<Db>, row: FullLevelRow) -> sqlx::Result<Db::QueryResult>;
}

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

impl LevelsRow for LeaderboardRow {
    fn user_id(&self) -> UserId {
        UserId::new(self.user_id as u64)
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

impl Default for RankRow {
    fn default() -> Self {
        Self { xp: 0, level: 1 }
    }
}

impl LevelsRow for RankRow {
    fn user_id(&self) -> serenity::all::UserId {
        unreachable!("user_id is not available on RankRow")
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

impl Default for XpRow {
    fn default() -> Self {
        Self {
            xp: 0,
            level: 1,
            total_xp: 0,
        }
    }
}

impl LevelsRow for XpRow {
    fn user_id(&self) -> UserId {
        unreachable!("user_id is not available on XpRow")
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
            user_id: id.get() as i64,
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
        self.total_xp += rand_xp as i64;
        self.xp += rand_xp;

        let next_level_xp = level_up_xp(self.level());
        if self.xp >= next_level_xp {
            self.xp -= next_level_xp;
            self.level += 1;
            return Some(self.level);
        };

        None
    }
}

impl LevelsRow for FullLevelRow {
    fn user_id(&self) -> UserId {
        UserId::new(self.user_id as u64)
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
