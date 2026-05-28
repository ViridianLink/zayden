use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::work::{WorkManager, WorkRow};
use jiff_sqlx::Timestamp;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::modules::gambling::StaminaTable;
use crate::{BotState, Error, Result};

use super::goals::GoalsTable;

pub struct WorkTable;

#[async_trait]
impl WorkManager<Postgres> for WorkTable {
    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<WorkRow>> {
        let id = id.into();

        sqlx::query_as!(
            WorkRow,
            r#"SELECT
                g.user_id,
                g.coins,
                g.gems,
                g.stamina,

                COALESCE(l.level, 0) AS level,
                
                COALESCE(m.miners, 0) AS miners,
                COALESCE(m.prestige, 0) AS prestige,
                COALESCE(m.mine_activity, now()::TIMESTAMP) AS "mine_activity: jiff_sqlx::Timestamp"

                FROM gambling g
                LEFT JOIN levels l ON g.user_id = l.user_id
                LEFT JOIN gambling_mine m on g.user_id = m.user_id
                WHERE g.user_id = $1;"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: WorkRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (user_id, coins, gems, stamina)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems, stamina = EXCLUDED.stamina;",
            row.user_id,
            row.coins,
            row.gems,
            row.stamina
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            "INSERT INTO gambling_mine (user_id, mine_activity)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET
            mine_activity = EXCLUDED.mine_activity;",
            row.user_id,
            row.mine_activity as Option<Timestamp>,
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
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::work::<BotState, Postgres, StaminaTable, GoalsTable, WorkTable>(
            ctx,
            interaction,
            pool,
        )
        .await?;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        Commands::register_work()
    }
}
