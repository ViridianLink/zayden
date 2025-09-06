use async_trait::async_trait;
use gambling::commands::gift::GiftManager;
use gambling::{Commands, commands::gift::SenderRow};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::modules::gambling::{GamblingTable, GoalsTable};
use crate::{CtxData, Error, Result};

pub struct GiftTable;

#[async_trait]
impl GiftManager<Postgres> for GiftTable {
    async fn sender(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<SenderRow>> {
        let id = id.into();

        sqlx::query_as!(
            SenderRow,
            "SELECT
                g.id,
                g.coins,
                g.gems,
                g.gift,

                COALESCE(l.level, 0) AS level,
                
                m.prestige

                FROM gambling g
                LEFT JOIN levels l ON g.id = l.id
                LEFT JOIN gambling_mine m on g.id = m.id
                WHERE g.id = $1;",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save_sender(pool: &PgPool, row: SenderRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (id, coins, gems, gift)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems, gift = EXCLUDED.gift;",
            row.id,
            row.coins,
            row.gems,
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            "INSERT INTO levels (id, level)
            VALUES ($1, $2)
            ON CONFLICT (id) DO UPDATE SET
            level = EXCLUDED.level;",
            row.id,
            row.level,
        )
        .execute(&mut *tx)
        .await?;

        result.extend([result2]);

        tx.commit().await?;

        Ok(result)
    }
}

pub struct Gift;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Gift {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::gift::<CtxData, Postgres, GamblingTable, GoalsTable, GiftTable>(
            ctx,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_gift())
    }
}
