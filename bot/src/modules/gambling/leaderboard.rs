use async_trait::async_trait;
use futures::TryStreamExt;
use gambling::Commands;
use gambling::commands::leaderboard::{
    CoinsRow, EggplantsRow, GemsRow, HigherLowerRow, LeaderboardManager, LeaderboardRow,
    LottoTicketRow, NetworthRow, WeeklyHigherLowerRow,
};
use gambling::shop::{EGGPLANT, LOTTO_TICKET, WEAPON_CRATE};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{CtxData, Error, Result};

const LIMIT: i64 = 10;

pub struct LeaderboardTable;

#[async_trait]
impl LeaderboardManager<Postgres> for LeaderboardTable {
    async fn networth(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            NetworthRow,
            "sql/gambling/LeaderboardManager/networth.sql",
            global,
            users,
            EGGPLANT.id,
            EGGPLANT.coin_cost().unwrap_or_default(),
            WEAPON_CRATE.id,
            WEAPON_CRATE.coin_cost().unwrap_or_default(),
            LIMIT,
            offset
        )
        .fetch(pool)
        .map_ok(LeaderboardRow::NetWorth)
        .try_collect::<Vec<_>>()
        .await
    }

    async fn networth_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let user_id = id.into();

        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/networth_row_number.sql",
            global,
            users,
            user_id.get() as i64,
            EGGPLANT.id,
            EGGPLANT.coin_cost().unwrap_or_default(),
            WEAPON_CRATE.id,
            WEAPON_CRATE.coin_cost().unwrap_or_default()
        )
        .fetch_optional(pool)
        .await
        .map(|num_opt_opt| num_opt_opt.flatten())
    }

    async fn coins(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            CoinsRow,
            "sql/gambling/LeaderboardManager/coins.sql",
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

    async fn coins_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let user_id = id.into();

        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/coins_row_number.sql",
            global,
            users,
            user_id.get() as i64
        )
        .fetch_optional(pool)
        .await
        .map(|num| num.flatten())
    }

    async fn gems(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            GemsRow,
            "sql/gambling/LeaderboardManager/gems.sql",
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

    async fn gems_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let user_id = id.into();

        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/gems_row_number.sql",
            global,
            users,
            user_id.get() as i64
        )
        .fetch_optional(pool)
        .await
        .map(|num| num.flatten())
    }

    async fn eggplants(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            EggplantsRow,
            "sql/gambling/LeaderboardManager/item.sql",
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

    async fn eggplants_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let id = id.into();

        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/item_row_number.sql",
            global,
            users,
            EGGPLANT.id,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
        .map(|num| num.flatten())
    }

    async fn lottotickets(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            LottoTicketRow,
            "sql/gambling/LeaderboardManager/item.sql",
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

    async fn lottotickets_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let id = id.into();

        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/item_row_number.sql",
            global,
            users,
            LOTTO_TICKET.id,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
        .map(|num| num.flatten())
    }

    async fn higherlower(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            HigherLowerRow,
            "sql/gambling/LeaderboardManager/higherlower.sql",
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

    async fn higherlower_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let id = id.into();

        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/higherlower_row_number.sql",
            global,
            users,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
        .map(|num| num.flatten())
    }

    async fn weekly_higherlower(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>> {
        let offset = (page_num - 1) * LIMIT;

        sqlx::query_file_as!(
            WeeklyHigherLowerRow,
            "sql/gambling/LeaderboardManager/weekly_higherlower.sql",
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

    async fn weekly_higherlower_row_number(
        pool: &PgPool,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let id = id.into();

        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/weekly_higherlower_row_number.sql",
            global,
            users,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
        .map(|num| num.flatten())
    }
}

pub struct Leaderboard;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Leaderboard {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::leaderboard::<CtxData, Postgres, LeaderboardTable>(
            ctx,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_leaderboard())
    }
}
