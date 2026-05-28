use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::send::{SendManager, SendRow};
use serenity::all::{CreateCommand, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;
use crate::bindings::gambling::{GamblingTable, StaminaTable};

use super::goals::GoalsTable;

pub struct SendTable;

#[async_trait]
impl SendManager<Postgres> for SendTable {
    async fn row(
        pool: &PgPool,
        id: impl Into<UserId> + std::marker::Send,
    ) -> sqlx::Result<Option<SendRow>> {
        let id = id.into();

        sqlx::query_as!(
            SendRow,
            "SELECT
                g.user_id,
                g.coins,
                g.gems,
                g.stamina,

                COALESCE(l.level, 0) AS level,
                
                m.prestige

                FROM gambling g
                LEFT JOIN levels l ON g.user_id = l.user_id
                LEFT JOIN gambling_mine m on g.user_id = m.user_id
                WHERE g.user_id = $1;",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: SendRow) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO gambling (user_id, coins, gems, stamina)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems, stamina = EXCLUDED.stamina;",
            row.user_id,
            row.coins,
            row.gems,
            row.stamina
        )
        .execute(pool)
        .await
    }
}

pub struct Send;

#[async_trait]
impl ModuleCommand for Send {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("send")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_send()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::send::<BotState, Postgres, GamblingTable, StaminaTable, GoalsTable, SendTable>(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await
        .map_err(HandlerError::from_respond)
    }
}
