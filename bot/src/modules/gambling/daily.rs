use async_trait::async_trait;
use gambling::{
    Commands,
    commands::daily::{DailyManager, DailyRow},
};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::{PgPool, Postgres, postgres::PgQueryResult};
use zayden_core::ApplicationCommand;

use crate::{Error, Result};

pub struct DailyTable;

#[async_trait]
impl DailyManager<Postgres> for DailyTable {
    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<DailyRow>> {
        let id = id.into();

        sqlx::query_as!(
            DailyRow,
            "SELECT
                g.id,
                g.coins,
                g.daily,
                
                COALESCE(m.prestige, 0) as prestige

                FROM gambling g
                LEFT JOIN gambling_mine m on g.id = m.id
                WHERE g.id = $1;",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: DailyRow) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO gambling (id, coins, daily)
            VALUES ($1, $2, now())
            ON CONFLICT (id) DO UPDATE SET
            coins = EXCLUDED.coins, daily = EXCLUDED.daily;",
            row.id,
            row.coins,
        )
        .execute(pool)
        .await
    }
}

pub struct Daily;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Daily {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::daily::<Postgres, DailyTable>(&ctx.http, interaction, pool).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_daily())
    }
}
