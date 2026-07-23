use std::borrow::Cow;

use async_trait::async_trait;
use futures::TryStreamExt;
use gambling::Commands;
use gambling::common::leaderboard::{
    CoinsRow,
    EggplantsRow,
    GemsRow,
    HigherLowerRow,
    LottoTicketRow,
    WeeklyHigherLowerRow,
};
use gambling::common::{LeaderboardManager, LeaderboardRow};
use gambling::shop::{EGGPLANT, LOTTO_TICKET};
use serenity::all::{CreateCommand, UserId};
use sqlx::{PgPool, Postgres};
use zayden_core::as_i64;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

use crate::BotState;

const LIMIT: i64 = 10;

pub struct LeaderboardTable;

#[async_trait]
impl LeaderboardManager<Postgres> for LeaderboardTable {
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
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        let user_id = id;

        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/coins_row_number.sql",
            global,
            users,
            as_i64(user_id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
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
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        let user_id = id;

        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/gems_row_number.sql",
            global,
            users,
            as_i64(user_id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
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
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/item_row_number.sql",
            global,
            users,
            EGGPLANT.id,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
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
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/item_row_number.sql",
            global,
            users,
            LOTTO_TICKET.id,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
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
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/higherlower_row_number.sql",
            global,
            users,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
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
        id: UserId,
    ) -> sqlx::Result<Option<i64>> {
        sqlx::query_file_scalar!(
            "sql/gambling/LeaderboardManager/weekly_higherlower_row_number.sql",
            global,
            users,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
    }
}

pub struct Leaderboard;

#[async_trait]
impl ModuleCommand for Leaderboard {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("leaderboard")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_leaderboard()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::leaderboard::<BotState, Postgres, LeaderboardTable>(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for Leaderboard {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("leaderboard"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        gambling::Leaderboard::run_component::<BotState, Postgres, LeaderboardTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
