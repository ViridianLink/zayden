use futures::TryStreamExt;
use serenity::all::{Mentionable, UserId};
use sqlx::{FromRow, PgPool};
use zayden_core::{EmojiCache, FormatNum, as_i64, as_u64};

use crate::shop::{EGGPLANT, LOTTO_TICKET};
use crate::{Coins, Gems, Result};

const LIMIT: i64 = 10;

pub struct LeaderboardManager;

impl LeaderboardManager {
    pub async fn coins(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            CoinsRow,
            "sql/LeaderboardManager/coins.sql",
            global,
            users,
            LIMIT,
            offset
        )
        .fetch(pool)
        .map_ok(LeaderboardRow::Coins)
        .try_collect::<Vec<_>>()
        .await
    }

    pub async fn coins_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        let user_id = id;

        sqlx::query_file_scalar!(
            "sql/LeaderboardManager/coins_row_number.sql",
            global,
            users,
            as_i64(user_id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
    }

    pub async fn gems(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            GemsRow,
            "sql/LeaderboardManager/gems.sql",
            global,
            users,
            LIMIT,
            offset
        )
        .fetch(pool)
        .map_ok(LeaderboardRow::Gems)
        .try_collect::<Vec<_>>()
        .await
    }

    pub async fn gems_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        let user_id = id;

        sqlx::query_file_scalar!(
            "sql/LeaderboardManager/gems_row_number.sql",
            global,
            users,
            as_i64(user_id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
    }

    pub async fn eggplants(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            EggplantsRow,
            "sql/LeaderboardManager/item.sql",
            global,
            users,
            EGGPLANT.id,
            LIMIT,
            offset
        )
        .fetch(pool)
        .map_ok(LeaderboardRow::Eggplants)
        .try_collect::<Vec<_>>()
        .await
    }

    pub async fn eggplants_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        sqlx::query_file_scalar!(
            "sql/LeaderboardManager/item_row_number.sql",
            global,
            users,
            EGGPLANT.id,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
    }

    pub async fn lottotickets(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            LottoTicketRow,
            "sql/LeaderboardManager/item.sql",
            global,
            users,
            LOTTO_TICKET.id,
            LIMIT,
            offset
        )
        .fetch(pool)
        .map_ok(LeaderboardRow::LottoTickets)
        .try_collect::<Vec<_>>()
        .await
    }

    pub async fn lottotickets_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        sqlx::query_file_scalar!(
            "sql/LeaderboardManager/item_row_number.sql",
            global,
            users,
            LOTTO_TICKET.id,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
    }

    pub async fn higherlower(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            HigherLowerRow,
            "sql/LeaderboardManager/higherlower.sql",
            global,
            users,
            LIMIT,
            offset
        )
        .fetch(pool)
        .map_ok(LeaderboardRow::HigherLower)
        .try_collect::<Vec<_>>()
        .await
    }

    pub async fn higherlower_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        sqlx::query_file_scalar!(
            "sql/LeaderboardManager/higherlower_row_number.sql",
            global,
            users,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
    }

    pub async fn weekly_higherlower(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            WeeklyHigherLowerRow,
            "sql/LeaderboardManager/weekly_higherlower.sql",
            global,
            users,
            LIMIT,
            offset
        )
        .fetch(pool)
        .map_ok(LeaderboardRow::WeeklyHigherLower)
        .try_collect::<Vec<_>>()
        .await
    }

    pub async fn weekly_higherlower_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        sqlx::query_file_scalar!(
            "sql/LeaderboardManager/weekly_higherlower_row_number.sql",
            global,
            users,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
    }
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

    pub fn as_desc(&self, emojis: &EmojiCache, i: usize) -> Result<String> {
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
                format!("{} {}", row.quantity.format(), EGGPLANT.emoji(emojis)?)
            },
            Self::LottoTickets(row) => {
                format!("{} {}", row.quantity.format(), LOTTO_TICKET.emoji(emojis)?)
            },
            Self::HigherLower(row) => row.higher_or_lower_score.to_string(),
            Self::WeeklyHigherLower(row) => {
                row.weekly_higher_or_lower_score.to_string()
            },
        };

        Ok(format!("{place} - {} - {data}", self.user_id().mention()))
    }
}

pub async fn get_rows(
    leaderboard: &str,
    pool: &PgPool,
    users: Option<&[i64]>,
    page_num: i64,
) -> sqlx::Result<Vec<LeaderboardRow>> {
    let global = users.is_none();
    let users = users.unwrap_or_default();

    match leaderboard {
        "coins" => LeaderboardManager::coins(pool, global, users, page_num).await,
        "gems" => LeaderboardManager::gems(pool, global, users, page_num).await,
        "eggplants" => {
            LeaderboardManager::eggplants(pool, global, users, page_num).await
        },
        "lottotickets" => {
            LeaderboardManager::lottotickets(pool, global, users, page_num).await
        },
        "higherlower" => {
            LeaderboardManager::higherlower(pool, global, users, page_num).await
        },
        "weekly_higherlower" => {
            LeaderboardManager::weekly_higherlower(pool, global, users, page_num)
                .await
        },
        _ => Ok(Vec::new()),
    }
}

pub async fn get_row_number(
    leaderboard: &str,
    pool: &PgPool,
    users: Option<&[i64]>,
    user: UserId,
) -> sqlx::Result<Option<i64>> {
    let global = users.is_none();
    let users = users.unwrap_or_default();

    match leaderboard {
        "coins" => {
            LeaderboardManager::coins_row_number(pool, global, users, user).await
        },
        "gems" => {
            LeaderboardManager::gems_row_number(pool, global, users, user).await
        },
        "eggplants" => {
            LeaderboardManager::eggplants_row_number(pool, global, users, user).await
        },
        "lottotickets" => {
            LeaderboardManager::lottotickets_row_number(pool, global, users, user)
                .await
        },
        "higherlower" => {
            LeaderboardManager::higherlower_row_number(pool, global, users, user)
                .await
        },
        "weekly_higherlower" => {
            LeaderboardManager::weekly_higherlower_row_number(
                pool, global, users, user,
            )
            .await
        },
        _ => Ok(None),
    }
}
