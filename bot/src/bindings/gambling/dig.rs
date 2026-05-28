use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::dig::{DigManager, DigRow};
use jiff_sqlx::Timestamp;
use serenity::all::{CreateCommand, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;
use crate::bindings::gambling::StaminaTable;

use super::GoalsTable;

pub struct DigTable;

#[async_trait]
impl DigManager<Postgres> for DigTable {
    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<DigRow>> {
        let id = id.into();

        sqlx::query_as!(
            DigRow,
            r#"SELECT
                g.user_id,
                g.coins,
                g.gems,
                g.stamina,

                COALESCE(l.level, 0) AS "level!",

                COALESCE(m.miners, 0) AS "miners!",
                COALESCE(m.coal, 0) AS "coal!",
                COALESCE(m.iron, 0) AS "iron!",
                COALESCE(m.gold, 0) AS "gold!",
                COALESCE(m.redstone, 0) AS "redstone!",
                COALESCE(m.lapis, 0) AS "lapis!",
                COALESCE(m.diamonds, 0) AS "diamonds!",
                COALESCE(m.emeralds, 0) AS "emeralds!",
                COALESCE(m.prestige, 0) AS "prestige!",
                COALESCE(m.mine_activity, now()::TIMESTAMP) AS "mine_activity!: jiff_sqlx::Timestamp"
                
            FROM gambling g
            LEFT JOIN levels l ON g.user_id = l.user_id
            LEFT JOIN gambling_mine m ON g.user_id = m.user_id
            WHERE g.user_id = $1;"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: &DigRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            r#"INSERT INTO gambling (user_id, coins, gems, stamina)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins,
            gems = EXCLUDED.gems,
            stamina = EXCLUDED.stamina"#,
            row.user_id,
            row.coins,
            row.gems,
            row.stamina,
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            r#"INSERT INTO gambling_mine (user_id, miners, coal, iron, gold, redstone, lapis, diamonds, emeralds, prestige, mine_activity)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (user_id) DO UPDATE SET
                miners = EXCLUDED.miners,
                coal = EXCLUDED.coal,
                iron = EXCLUDED.iron,
                gold = EXCLUDED.gold,
                redstone = EXCLUDED.redstone,
                lapis = EXCLUDED.lapis,
                diamonds = EXCLUDED.diamonds,
                emeralds = EXCLUDED.emeralds,
                prestige = EXCLUDED.prestige,
                mine_activity = EXCLUDED.mine_activity"#,
            row.user_id,
            row.miners,
            row.coal,
            row.iron,
            row.gold,
            row.redstone,
            row.lapis,
            row.diamonds,
            row.emeralds,
            row.prestige,
            row.mine_activity as Timestamp,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        result.extend([result2]);

        Ok(result)
    }
}

pub struct Dig;

#[async_trait]
impl ModuleCommand for Dig {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("dig")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_dig()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::dig::<BotState, Postgres, StaminaTable, GoalsTable, DigTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await
        .map_err(HandlerError::from_respond)
    }
}
