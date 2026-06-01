use std::borrow::Cow;

use async_trait::async_trait;
use bigdecimal::ToPrimitive;
use gambling::shop::LOTTO_TICKET;
use gambling::{Commands, LottoManager, LottoRow};
use serenity::all::{CreateCommand, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, Postgres};
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;
use crate::bindings::gambling::GamblingTable;

pub struct LottoTable;

#[async_trait]
impl LottoManager<Postgres> for LottoTable {
    async fn row(
        conn: &mut PgConnection,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<LottoRow>> {
        let id = id.into();

        sqlx::query_file_as!(
            LottoRow,
            "sql/gambling/LottoManager/row.sql",
            id.get().cast_signed(),
            LOTTO_TICKET.id
        )
        .fetch_optional(conn)
        .await
    }

    async fn rows(conn: &mut PgConnection) -> sqlx::Result<Vec<LottoRow>> {
        sqlx::query_file_as!(
            LottoRow,
            "sql/gambling/LottoManager/rows.sql",
            LOTTO_TICKET.id
        )
        .fetch_all(conn)
        .await
    }

    async fn total_tickets(conn: &mut PgConnection) -> sqlx::Result<i64> {
        sqlx::query_file_scalar!(
            "sql/gambling/LottoManager/total_tickets.sql",
            LOTTO_TICKET.id
        )
        .fetch_one(conn)
        .await
        .map(Option::unwrap_or_default)
        .map(|x| x.to_i64().unwrap_or_default())
    }

    async fn delete_tickets(conn: &mut PgConnection) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "sql/gambling/LottoManager/delete_tickets.sql",
            LOTTO_TICKET.id
        )
        .execute(conn)
        .await
    }
}

pub struct Lotto;

#[async_trait]
impl ModuleCommand for Lotto {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("lotto")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_lotto()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::lotto::<BotState, Postgres, GamblingTable, LottoTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
