use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::craft::{CraftManager, CraftRow};
use serenity::all::{CreateCommand, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::as_i64;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct CraftTable;

#[async_trait]
impl CraftManager<Postgres> for CraftTable {
    async fn row(pool: &PgPool, id: UserId) -> sqlx::Result<Option<CraftRow>> {
        sqlx::query_file_as!(
            CraftRow,
            "sql/gambling/CraftManager/craft-row.sql",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: CraftRow) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO gambling_mine (user_id, coal, iron, gold, redstone, lapis, diamonds, emeralds, tech, utility, production)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (user_id) DO UPDATE SET
            coal = EXCLUDED.coal,
            iron = EXCLUDED.iron,
            gold = EXCLUDED.gold,
            redstone = EXCLUDED.redstone,
            lapis = EXCLUDED.lapis,
            diamonds = EXCLUDED.diamonds,
            emeralds = EXCLUDED.emeralds,
            tech = EXCLUDED.tech,
            utility = EXCLUDED.utility,
            production = EXCLUDED.production;",
            row.user_id,
            row.coal,
            row.iron,
            row.gold,
            row.redstone,
            row.lapis,
            row.diamonds,
            row.emeralds,
            row.tech,
            row.utility,
            row.production,
        )
        .execute(pool)
        .await
    }
}

pub struct Craft;

#[async_trait]
impl ModuleCommand for Craft {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("craft")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_craft()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::craft::<BotState, Postgres, CraftTable>(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
