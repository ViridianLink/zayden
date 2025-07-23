use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::craft::{CraftManager, CraftRow};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::SlashCommand;

use crate::{Error, Result};

pub struct CraftTable;

#[async_trait]
impl CraftManager<Postgres> for CraftTable {
    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<CraftRow>> {
        let id = id.into();

        sqlx::query_as!(CraftRow, "SELECT id, coal, iron, gold, redstone, lapis, diamonds, emeralds, tech, utility, production FROM gambling_mine WHERE id = $1", id.get() as i64).fetch_optional(pool).await
    }

    async fn save(pool: &PgPool, row: CraftRow) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO gambling_mine (id, coal, iron, gold, redstone, lapis, diamonds, emeralds, tech, utility, production)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
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
            row.id,
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
impl SlashCommand<Error, Postgres> for Craft {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::craft::<Postgres, CraftTable>(&ctx.http, interaction, options, pool).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_craft())
    }
}
