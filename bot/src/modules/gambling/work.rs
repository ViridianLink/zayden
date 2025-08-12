use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::work::{WorkManager, WorkRow};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::modules::gambling::StaminaTable;
use crate::{Error, Result};

use super::goals::GoalsTable;

pub struct WorkTable;

#[async_trait]
impl WorkManager<Postgres> for WorkTable {
    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<WorkRow>> {
        let id = id.into();

        sqlx::query_as!(
            WorkRow,
            "SELECT
                g.id,
                g.coins,
                g.gems,
                g.stamina,

                COALESCE(l.level, 0) AS level,
                
                COALESCE(m.miners, 0) AS miners,
                COALESCE(m.prestige, 0) AS prestige,
                COALESCE(m.mine_activity, now()::TIMESTAMP) AS mine_activity

                FROM gambling g
                LEFT JOIN levels l ON g.id = l.id
                LEFT JOIN gambling_mine m on g.id = m.id
                WHERE g.id = $1;",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: WorkRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (id, coins, gems, stamina)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems, stamina = EXCLUDED.stamina;",
            row.id,
            row.coins,
            row.gems,
            row.stamina
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            "INSERT INTO gambling_mine (id, mine_activity)
            VALUES ($1, $2)
            ON CONFLICT (id) DO UPDATE SET
            mine_activity = EXCLUDED.mine_activity;",
            row.id,
            row.mine_activity,
        )
        .execute(&mut *tx)
        .await?;

        result.extend([result2]);

        tx.commit().await?;

        Ok(result)
    }
}

pub struct Work;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Work {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::work::<Postgres, StaminaTable, GoalsTable, WorkTable>(
            &ctx.http,
            interaction,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_work())
    }
}
