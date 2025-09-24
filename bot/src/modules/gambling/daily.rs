use async_trait::async_trait;
use chrono::NaiveDate;
use gambling::commands::daily::{DailyManager, DailyRow};
use gambling::commands::goals::GoalsRow;
use gambling::{Commands, GamblingGoalsRow, GoalsManager};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{CtxData, Error, Result};

pub struct DailyTable;

#[async_trait]
impl DailyManager<Postgres> for DailyTable {
    async fn daily_row(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<DailyRow>> {
        let id = id.into();

        sqlx::query_file_as!(
            DailyRow,
            "sql/gambling/DailyManager/daily_row.sql",
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

#[async_trait]
impl GoalsManager<Postgres> for DailyTable {
    async fn row(_pool: &PgPool, _id: impl Into<UserId> + Send) -> sqlx::Result<Option<GoalsRow>> {
        unimplemented!()
    }

    async fn full_rows(
        _pool: &PgPool,
        _id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>> {
        unimplemented!()
    }

    async fn update(
        pool: &PgPool,
        rows: &[GamblingGoalsRow],
    ) -> sqlx::Result<Vec<GamblingGoalsRow>> {
        let user_id = match rows.first() {
            Some(row) => row.user_id,
            None => return Ok(Vec::new()),
        };

        let mut tx = pool.begin().await?;

        sqlx::query!("DELETE FROM gambling_goals WHERE user_id = $1", user_id)
            .execute(&mut *tx)
            .await?;

        let num_rows = rows.len();
        let mut user_ids: Vec<i64> = Vec::with_capacity(num_rows);
        let mut goal_ids: Vec<String> = Vec::with_capacity(num_rows);
        let mut days: Vec<NaiveDate> = Vec::with_capacity(num_rows);
        let mut progresses: Vec<i64> = Vec::with_capacity(num_rows);
        let mut targets: Vec<i64> = Vec::with_capacity(num_rows);

        for row in rows {
            user_ids.push(row.user_id);
            goal_ids.push(row.goal_id.clone());
            days.push(row.day);
            progresses.push(row.progress);
            targets.push(row.target);
        }

        let rows = sqlx::query_as!(
            GamblingGoalsRow,
            "INSERT INTO gambling_goals (user_id, goal_id, day, progress, target)
            SELECT * FROM UNNEST($1::bigint[], $2::text[], $3::date[], $4::bigint[], $5::bigint[])
            RETURNING user_id, goal_id, day, progress, target;",
            &user_ids,
            &goal_ids,
            &days,
            &progresses,
            &targets
        )
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(rows)
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
        Commands::daily::<CtxData, Postgres, DailyTable>(ctx, interaction, pool).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_daily())
    }
}
