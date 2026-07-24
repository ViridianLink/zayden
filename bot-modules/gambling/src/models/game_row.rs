use serenity::all::UserId;
use sqlx::postgres::PgQueryResult;
use sqlx::{FromRow, PgPool};
use zayden_core::as_i64;

use super::{Coins, Gems, MaxBet};
use crate::{Prestige, START_AMOUNT};

#[derive(FromRow)]
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

    pub async fn save(pool: &PgPool, row: Self) -> sqlx::Result<PgQueryResult> {
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
