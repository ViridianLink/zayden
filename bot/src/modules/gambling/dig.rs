use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::dig::{DigManager, DigRow};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::modules::gambling::StaminaTable;
use crate::{Error, Result};

use super::GoalsTable;

pub struct DigTable;

#[async_trait]
impl DigManager<Postgres> for DigTable {
    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<DigRow>> {
        let id = id.into();

        sqlx::query_as!(
            DigRow,
            "SELECT
                g.id,
                g.coins,
                g.gems,
                g.stamina,

                COALESCE(l.level, 0) AS level,

                COALESCE(m.miners, 0) AS miners,
                COALESCE(m.coal, 0) AS coal,
                COALESCE(m.iron, 0) AS iron,
                COALESCE(m.gold, 0) AS gold,
                COALESCE(m.redstone, 0) AS redstone,
                COALESCE(m.lapis, 0) AS lapis,
                COALESCE(m.diamonds, 0) AS diamonds,
                COALESCE(m.emeralds, 0) AS emeralds,
                COALESCE(m.prestige, 0) AS prestige,
                COALESCE(m.mine_activity, now()::TIMESTAMP) AS mine_activity
                
            FROM gambling g
            LEFT JOIN levels l ON g.id = l.id
            LEFT JOIN gambling_mine m ON g.id = m.id
            WHERE g.id = $1;",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: DigRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (id, coins, gems, stamina)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
            coins = EXCLUDED.coins,
            gems = EXCLUDED.gems,
            stamina = EXCLUDED.stamina;",
            row.id,
            row.coins,
            row.gems,
            row.stamina,
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            "INSERT INTO gambling_mine (id, coal, iron, gold, redstone, lapis, diamonds, emeralds, mine_activity)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
            coal = EXCLUDED.coal,
            iron = EXCLUDED.iron,
            gold = EXCLUDED.gold,
            redstone = EXCLUDED.redstone,
            lapis = EXCLUDED.lapis,
            diamonds = EXCLUDED.diamonds,
            emeralds = EXCLUDED.emeralds,
            mine_activity = EXCLUDED.mine_activity;",
            row.id,
            row.coal,
            row.iron,
            row.gold,
            row.redstone,
            row.lapis,
            row.diamonds,
            row.emeralds,
            row.mine_activity
        )
        .execute(&mut *tx)
        .await?;

        result.extend([result2]);

        tx.commit().await.unwrap();

        Ok(result)
    }
}

pub struct Dig;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Dig {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::dig::<Postgres, StaminaTable, GoalsTable, DigTable>(&ctx.http, interaction, pool)
            .await?;
        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_dig())
    }
}
