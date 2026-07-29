use serenity::all::UserId;
use sqlx::{FromRow, PgConnection, PgPool};
use zayden_core::as_i64;

use super::{Coins, Gems, MaxBet};
use crate::{Prestige, START_AMOUNT};

#[derive(Debug, Clone, FromRow)]
pub struct GameRow {
    pub user_id: i64,
    pub coins: i64,
    pub gems: i64,
    pub level: Option<i32>,
    pub prestige: Option<i64>,
}

impl GameRow {
    #[must_use]
    pub const fn new(id: UserId) -> Self {
        Self {
            user_id: as_i64(id.get()),
            coins: START_AMOUNT,
            gems: 0,
            level: Some(0),
            prestige: Some(0),
        }
    }

    pub async fn get(pool: &PgPool, id: UserId) -> sqlx::Result<Option<Self>> {
        sqlx::query_file_as!(Self, "sql/GameManager/row.sql", as_i64(id.get()))
            .fetch_optional(pool)
            .await
    }

    pub async fn ensure(pool: &PgPool, id: UserId) -> sqlx::Result<()> {
        let mut conn = pool.acquire().await?;

        Self::insert_missing(&mut conn, as_i64(id.get())).await
    }

    async fn insert_missing(
        conn: &mut PgConnection,
        user_id: i64,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO gambling (user_id) VALUES ($1)
            ON CONFLICT (user_id) DO NOTHING;",
            user_id
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    pub async fn commit_tx(
        conn: &mut PgConnection,
        id: UserId,
        delta: &GameDelta,
    ) -> sqlx::Result<Option<GameCommit>> {
        let user_id = as_i64(id.get());

        Self::insert_missing(&mut *conn, user_id).await?;

        sqlx::query_as!(
            GameCommit,
            "UPDATE gambling SET
                coins = coins + $2,
                gems = gems + $3
            WHERE user_id = $1
                AND ($2::bigint = 0 OR coins + $2 >= 0)
                AND ($3::bigint = 0 OR gems + $3 >= 0)
            RETURNING coins, gems;",
            user_id,
            delta.coins,
            delta.gems,
        )
        .fetch_optional(conn)
        .await
    }

    pub async fn commit(
        pool: &PgPool,
        id: UserId,
        delta: &GameDelta,
    ) -> sqlx::Result<Option<GameCommit>> {
        let mut tx = pool.begin().await?;

        let Some(commit) = Self::commit_tx(&mut tx, id, delta).await? else {
            tx.rollback().await?;
            return Ok(None);
        };

        tx.commit().await?;

        Ok(Some(commit))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameDelta {
    pub coins: i64,
    pub gems: i64,
}

impl GameDelta {
    #[must_use]
    pub const fn between(before: &GameRow, after: &GameRow) -> Self {
        Self { coins: after.coins - before.coins, gems: after.gems - before.gems }
    }

    #[must_use]
    pub const fn coins(coins: i64) -> Self {
        Self { coins, gems: 0 }
    }
}

#[derive(Debug, FromRow)]
pub struct GameCommit {
    pub coins: i64,
    pub gems: i64,
}

impl Coins for GameRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for GameRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl Prestige for GameRow {
    fn prestige(&self) -> i64 {
        self.prestige.unwrap_or_default()
    }
}

impl MaxBet for GameRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}
