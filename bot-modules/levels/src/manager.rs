use jiff_sqlx::{Timestamp, ToSqlx};
use serenity::all::{GuildId, UserId};
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use sqlx::prelude::FromRow;
use zayden_core::{as_i64, as_u64};

use crate::level_up_xp;

fn accrue_message(
    xp: &mut i32,
    level: &mut i32,
    total_xp: &mut i64,
    message_count: &mut i64,
) -> Option<i32> {
    *message_count += 1;

    let rand_xp = rand::random_range(15..25);
    *total_xp += i64::from(rand_xp);
    *xp += rand_xp;

    let next_level_xp = level_up_xp(*level);
    if *xp >= next_level_xp {
        *xp -= next_level_xp;
        *level += 1;
        return Some(*level);
    }

    None
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

impl LeaderboardRow {
    pub async fn guild_leaderboard(
        pool: &PgPool,
        guild_id: GuildId,
        page: i64,
    ) -> sqlx::Result<Vec<Self>> {
        let offset = (page - 1) * 10;

        sqlx::query_as!(
            Self,
            "SELECT user_id, xp, level, message_count FROM guild_levels WHERE guild_id = $1 ORDER BY level DESC, xp DESC LIMIT 10 OFFSET $2",
            as_i64(guild_id.get()),
            offset
        )
        .fetch_all(pool)
        .await
    }

    pub async fn global_leaderboard(
        pool: &PgPool,
        page: i64,
    ) -> sqlx::Result<Vec<Self>> {
        let offset = (page - 1) * 10;

        sqlx::query_as!(
            Self,
            "SELECT user_id, xp, level, message_count FROM levels ORDER BY level DESC, xp DESC LIMIT 10 OFFSET $1",
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
    pub async fn get(pool: &PgPool, id: UserId) -> sqlx::Result<Option<Self>> {
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
        user_id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        let id = as_i64(user_id.get());

        sqlx::query_scalar!(
    "SELECT row_number FROM (SELECT user_id, ROW_NUMBER() OVER (ORDER BY level DESC, xp DESC) FROM levels) AS ranked WHERE user_id = $1",
    id
)
        .fetch_one(pool)
        .await
    }

    pub async fn guild_get(
        pool: &PgPool,
        guild_id: GuildId,
        id: UserId,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT xp, level FROM guild_levels WHERE guild_id = $1 AND user_id = $2",
            as_i64(guild_id.get()),
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn guild_user_rank(
        pool: &PgPool,
        guild_id: GuildId,
        user_id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        let rank = sqlx::query_scalar!(
    "SELECT row_number FROM (SELECT user_id, ROW_NUMBER() OVER (ORDER BY level DESC, xp DESC) FROM guild_levels WHERE guild_id = $1) AS ranked WHERE user_id = $2",
    as_i64(guild_id.get()),
    as_i64(user_id.get())
)
        .fetch_optional(pool)
        .await?;

        Ok(rank.flatten())
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
    pub async fn get(pool: &PgPool, id: UserId) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT xp, level, total_xp FROM levels WHERE user_id = $1",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn guild_get(
        pool: &PgPool,
        guild_id: GuildId,
        id: UserId,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT xp, level, total_xp FROM guild_levels WHERE guild_id = $1 AND user_id = $2",
            as_i64(guild_id.get()),
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
    #[must_use]
    pub fn new(id: UserId) -> Self {
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
        accrue_message(
            &mut self.xp,
            &mut self.level,
            &mut self.total_xp,
            &mut self.message_count,
        )
    }

    pub async fn get(pool: &PgPool, id: UserId) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"SELECT user_id, xp, level, total_xp, message_count, last_xp as "last_xp: jiff_sqlx::Timestamp" FROM levels WHERE user_id = $1"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

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
            i32::try_from(self.total_xp).unwrap_or(i32::MAX),
            self.level,
            i32::try_from(self.message_count).unwrap_or(i32::MAX),
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

#[derive(FromRow)]
pub struct GuildLevelRow {
    pub guild_id: i64,
    pub user_id: i64,
    pub xp: i32,
    pub level: i32,
    pub total_xp: i64,
    pub message_count: i64,
    pub last_xp: Timestamp,
}

impl GuildLevelRow {
    #[must_use]
    pub fn new(guild_id: GuildId, id: UserId) -> Self {
        Self {
            guild_id: as_i64(guild_id.get()),
            user_id: as_i64(id.get()),
            xp: 0,
            level: 0,
            total_xp: 0,
            message_count: 0,
            last_xp: jiff::Timestamp::default().to_sqlx(),
        }
    }

    pub fn new_message(&mut self) -> Option<i32> {
        accrue_message(
            &mut self.xp,
            &mut self.level,
            &mut self.total_xp,
            &mut self.message_count,
        )
    }

    pub async fn get(
        pool: &PgPool,
        guild_id: GuildId,
        id: UserId,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"SELECT guild_id, user_id, xp, level, total_xp, message_count, last_xp as "last_xp: jiff_sqlx::Timestamp" FROM guild_levels WHERE guild_id = $1 AND user_id = $2"#,
            as_i64(guild_id.get()),
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn save(self, pool: &PgPool) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            self.guild_id
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, 'PLACEHOLDER') ON CONFLICT (id) DO NOTHING",
            self.user_id
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "INSERT INTO guild_levels (guild_id, user_id, xp, total_xp, level, message_count, last_xp)
            VALUES ($1, $2, $3, $4, $5, $6, now())
            ON CONFLICT (guild_id, user_id) DO UPDATE
            SET xp = EXCLUDED.xp,
                total_xp = EXCLUDED.total_xp,
                level = EXCLUDED.level,
                message_count = EXCLUDED.message_count,
                last_xp = now();",
            self.guild_id,
            self.user_id,
            self.xp,
            self.total_xp,
            self.level,
            self.message_count,
        )
        .execute(pool)
        .await
    }
}

impl LevelsRow for GuildLevelRow {
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
