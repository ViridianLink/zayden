use async_trait::async_trait;
use gambling::commands::gift::GiftManager;
use gambling::{Commands, commands::gift::SenderRow};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::bindings::gambling::{GamblingTable, GoalsTable};
use crate::{BotState, Error, Result};

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
            r#"SELECT
                g.user_id,
                g.coins,
                g.gems,
                g.gift as "gift: jiff_sqlx::Timestamp",

                COALESCE(l.level, 0) AS "level!",
                
                m.prestige

                FROM gambling g
                LEFT JOIN levels l ON g.user_id = l.user_id
                LEFT JOIN gambling_mine m on g.user_id = m.user_id
                WHERE g.user_id = $1;"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save_sender(pool: &PgPool, row: SenderRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (user_id, coins, gems, gift)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems, gift = EXCLUDED.gift;",
            row.user_id,
            row.coins,
            row.gems,
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            "INSERT INTO levels (user_id, level)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET
            level = EXCLUDED.level;",
            row.user_id,
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
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::gift::<BotState, Postgres, GamblingTable, GoalsTable, GiftTable>(
            ctx,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        Commands::register_gift()
    }
}
