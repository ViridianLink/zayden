use async_trait::async_trait;
use serenity::all::UserId;
use sqlx::{Database, FromRow, Pool};
use zayden_core::as_i64;

use super::{Coins, Gems, MaxBet};
use crate::Prestige;

#[async_trait]
pub trait GameManager<Db: Database> {
    async fn row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<GameRow>>;

    async fn save(pool: &Pool<Db>, row: GameRow) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct GameRow {
    pub user_id: i64,
    pub coins: i64,
    pub gems: i64,
    pub level: Option<i32>,
    pub prestige: Option<i64>,
}

impl GameRow {
    pub fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            user_id: as_i64(id.get()),
            coins: 0,
            gems: 0,
            level: Some(0),
            prestige: Some(0),
        }
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
