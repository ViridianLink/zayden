use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::send::{SendManager, SendRow};
use serenity::all::{CreateCommand, UserId};
use sqlx::{PgPool, Postgres};
use zayden_core::as_i64;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use super::goals::GoalsTable;
use crate::BotState;
use crate::bindings::gambling::{GamblingTable, StaminaTable};

pub struct SendTable;

#[async_trait]
impl SendManager<Postgres> for SendTable {
    async fn row(pool: &PgPool, id: UserId) -> sqlx::Result<Option<SendRow>> {
        sqlx::query_as!(
            SendRow,
            r#"SELECT
                g.user_id,
                g.coins,
                g.gems,
                g.stamina,
                COALESCE(l.level, 0) AS "level!: i32",
                COALESCE(m.prestige, 0) AS "prestige!: i64"
                FROM gambling g
                LEFT JOIN levels l ON g.user_id = l.user_id
                LEFT JOIN gambling_mine m on g.user_id = m.user_id
                WHERE g.user_id = $1"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    async fn transfer(
        pool: &PgPool,
        sender: UserId,
        recipient: UserId,
        amount: i64,
    ) -> sqlx::Result<bool> {
        let mut tx = pool.begin().await?;

        let debit = sqlx::query!(
            "UPDATE gambling
            SET coins = coins - $2, stamina = GREATEST(stamina - 1, 0)
            WHERE user_id = $1 AND coins >= $2",
            as_i64(sender.get()),
            amount
        )
        .execute(&mut *tx)
        .await?;

        if debit.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query_file!(
            "./sql/gambling/GamblingManager/add_coins.sql",
            as_i64(recipient.get()),
            amount
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(true)
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
        Commands::send::<
            BotState,
            Postgres,
            GamblingTable,
            StaminaTable,
            GoalsTable,
            SendTable,
        >(cx.ctx, cx.interaction, options, &cx.app.db)
        .await?;
        Ok(())
    }
}
