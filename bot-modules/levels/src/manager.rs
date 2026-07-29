use jiff_sqlx::Timestamp;
use serenity::all::{GuildId, UserId};
use sqlx::PgPool;
use sqlx::prelude::FromRow;
use zayden_core::{as_i64, as_u64};

use crate::level_up_xp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageXp(i32);

impl MessageXp {
    pub const MAX: i32 = 24;
    pub const MIN: i32 = 15;

    #[must_use]
    pub fn roll() -> Self {
        Self(rand::random_range(Self::MIN..=Self::MAX))
    }

    #[must_use]
    pub const fn new(xp: i32) -> Self {
        Self(xp)
    }

    #[must_use]
    pub const fn amount(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelUp {
    pub from_level: i32,
    pub threshold: i32,
}

impl LevelUp {
    #[must_use]
    pub const fn check(xp: i32, level: i32) -> Option<Self> {
        let threshold = level_up_xp(level);

        if xp >= threshold {
            Some(Self { from_level: level, threshold })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn new_level(self) -> i32 {
        self.from_level + 1
    }
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
    pub async fn accrue_message(
        pool: &PgPool,
        id: UserId,
        xp: MessageXp,
    ) -> sqlx::Result<Option<i32>> {
        let user_id = as_i64(id.get());

        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, 'PLACEHOLDER') ON CONFLICT (id) DO NOTHING",
            user_id
        )
        .execute(&mut *tx)
        .await?;

        let accrued = sqlx::query!(
            "INSERT INTO levels (user_id, xp, total_xp, level, message_count, last_xp)
            VALUES ($1, $2, $2, 0, 1, now())
            ON CONFLICT (user_id) DO UPDATE
            SET xp = levels.xp + $2,
                total_xp = levels.total_xp + $2,
                message_count = levels.message_count + 1,
                last_xp = now()
            WHERE levels.last_xp <= now() - interval '1 minute'
            RETURNING xp, level;",
            user_id,
            xp.amount(),
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(accrued) = accrued else {
            tx.rollback().await?;
            return Ok(None);
        };

        let Some(level_up) = LevelUp::check(accrued.xp, accrued.level) else {
            tx.commit().await?;
            return Ok(None);
        };

        let new_level = sqlx::query_scalar!(
            "UPDATE levels
            SET xp = xp - $2, level = level + 1
            WHERE user_id = $1 AND level = $3 AND xp >= $2
            RETURNING level;",
            user_id,
            level_up.threshold,
            level_up.from_level,
        )
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(new_level)
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
    pub async fn accrue_message(
        pool: &PgPool,
        guild_id: GuildId,
        id: UserId,
        xp: MessageXp,
    ) -> sqlx::Result<Option<i32>> {
        let guild_id = as_i64(guild_id.get());
        let user_id = as_i64(id.get());

        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, 'PLACEHOLDER') ON CONFLICT (id) DO NOTHING",
            user_id
        )
        .execute(&mut *tx)
        .await?;

        let accrued = sqlx::query!(
            "INSERT INTO guild_levels (guild_id, user_id, xp, total_xp, level, message_count, last_xp)
            VALUES ($1, $2, $3::int, $3::int, 0, 1, now())
            ON CONFLICT (guild_id, user_id) DO UPDATE
            SET xp = guild_levels.xp + $3::int,
                total_xp = guild_levels.total_xp + $3::int,
                message_count = guild_levels.message_count + 1,
                last_xp = now()
            WHERE guild_levels.last_xp <= now() - interval '1 minute'
            RETURNING xp, level;",
            guild_id,
            user_id,
            xp.amount(),
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(accrued) = accrued else {
            tx.rollback().await?;
            return Ok(None);
        };

        let Some(level_up) = LevelUp::check(accrued.xp, accrued.level) else {
            tx.commit().await?;
            return Ok(None);
        };

        let new_level = sqlx::query_scalar!(
            "UPDATE guild_levels
            SET xp = xp - $3, level = level + 1
            WHERE guild_id = $1 AND user_id = $2 AND level = $4 AND xp >= $3
            RETURNING level;",
            guild_id,
            user_id,
            level_up.threshold,
            level_up.from_level,
        )
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(new_level)
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
