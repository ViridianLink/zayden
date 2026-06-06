use async_trait::async_trait;
use serenity::all::{Mentionable, UserId};
use sqlx::{Database, FromRow, Pool};
use zayden_core::{EmojiCache, FormatNum, as_u64};

use crate::shop::{EGGPLANT, LOTTO_TICKET};
use crate::{Coins, Gems};

#[async_trait]
pub trait LeaderboardManager<Db: Database> {
    async fn coins(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn coins_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn gems(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn gems_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn eggplants(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn eggplants_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn lottotickets(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn lottotickets_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn higherlower(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn higherlower_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn weekly_higherlower(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn weekly_higherlower_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;
}

#[derive(FromRow)]
pub struct CoinsRow {
    pub user_id: i64,
    pub coins: i64,
}

impl Coins for CoinsRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

#[derive(FromRow)]
pub struct GemsRow {
    pub user_id: i64,
    pub gems: i64,
}

impl Gems for GemsRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

#[derive(FromRow)]
pub struct EggplantsRow {
    pub user_id: i64,
    pub quantity: i64,
}

#[derive(FromRow)]
pub struct LottoTicketRow {
    pub user_id: i64,
    pub quantity: i64,
}

#[derive(FromRow)]
pub struct HigherLowerRow {
    pub user_id: i64,
    pub higher_or_lower_score: i32,
}

#[derive(FromRow)]
pub struct WeeklyHigherLowerRow {
    pub user_id: i64,
    pub weekly_higher_or_lower_score: i32,
}

pub enum LeaderboardRow {
    Coins(CoinsRow),
    Gems(GemsRow),
    Eggplants(EggplantsRow),
    LottoTickets(LottoTicketRow),
    HigherLower(HigherLowerRow),
    WeeklyHigherLower(WeeklyHigherLowerRow),
}

impl LeaderboardRow {
    #[must_use]
    pub const fn user_id(&self) -> UserId {
        match self {
            Self::Coins(row) => UserId::new(as_u64(row.user_id)),
            Self::Gems(row) => UserId::new(as_u64(row.user_id)),
            Self::Eggplants(row) => UserId::new(as_u64(row.user_id)),
            Self::LottoTickets(row) => UserId::new(as_u64(row.user_id)),
            Self::HigherLower(row) => UserId::new(as_u64(row.user_id)),
            Self::WeeklyHigherLower(row) => UserId::new(as_u64(row.user_id)),
        }
    }

    #[must_use]
    pub fn as_desc(&self, emojis: &EmojiCache, i: usize) -> String {
        let place = if i == 0 {
            "🥇".to_string()
        } else if i == 1 {
            "🥈".to_string()
        } else if i == 2 {
            "🥉".to_string()
        } else {
            format!("#{}", i + 1)
        };

        let data = match self {
            Self::Coins(row) => row.coins_str(),
            Self::Gems(row) => row.gems_str(),
            Self::Eggplants(row) => {
                format!("{} {}", row.quantity.format(), EGGPLANT.emoji(emojis))
            },
            Self::LottoTickets(row) => {
                format!("{} {}", row.quantity.format(), LOTTO_TICKET.emoji(emojis))
            },
            Self::HigherLower(row) => row.higher_or_lower_score.to_string(),
            Self::WeeklyHigherLower(row) => {
                row.weekly_higher_or_lower_score.to_string()
            },
        };

        format!("{place} - {} - {data}", self.user_id().mention())
    }
}

pub async fn get_rows<Db: Database, Manager: LeaderboardManager<Db>>(
    leaderboard: &str,
    pool: &Pool<Db>,
    users: Option<&[i64]>,
    page_num: i64,
) -> Vec<LeaderboardRow> {
    let global = users.is_none();
    let users = users.unwrap_or_default();

    match leaderboard {
        "coins" => {
            Manager::coins(pool, global, users, page_num).await.expect("async call")
        },
        "gems" => {
            Manager::gems(pool, global, users, page_num).await.expect("async call")
        },
        "eggplants" => Manager::eggplants(pool, global, users, page_num)
            .await
            .expect("async call"),
        "lottotickets" => Manager::lottotickets(pool, global, users, page_num)
            .await
            .expect("async call"),
        "higherlower" => Manager::higherlower(pool, global, users, page_num)
            .await
            .expect("async call"),
        "weekly_higherlower" => {
            Manager::weekly_higherlower(pool, global, users, page_num)
                .await
                .expect("async call")
        },
        _ => Vec::new(),
    }
}

pub async fn get_row_number<Db: Database, Manager: LeaderboardManager<Db>>(
    leaderboard: &str,
    pool: &Pool<Db>,
    users: Option<&[i64]>,
    user: UserId,
) -> Option<i64> {
    let global = users.is_none();
    let users = users.unwrap_or_default();

    match leaderboard {
        "coins" => Manager::coins_row_number(pool, global, users, user)
            .await
            .expect("async call"),
        "gems" => Manager::gems_row_number(pool, global, users, user)
            .await
            .expect("async call"),
        "eggplants" => Manager::eggplants_row_number(pool, global, users, user)
            .await
            .expect("async call"),
        "lottotickets" => {
            Manager::lottotickets_row_number(pool, global, users, user)
                .await
                .expect("async call")
        },
        "higherlower" => Manager::higherlower_row_number(pool, global, users, user)
            .await
            .expect("async call"),
        "weekly_higherlower" => {
            Manager::weekly_higherlower_row_number(pool, global, users, user)
                .await
                .expect("async call")
        },
        _ => None,
    }
}
