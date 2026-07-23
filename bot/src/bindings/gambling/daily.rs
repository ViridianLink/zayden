use std::borrow::Cow;

use async_trait::async_trait;
use gambling::commands::daily::{DailyManager, DailyRow};
use gambling::commands::goals::GoalsRow;
use gambling::{Commands, GamblingGoalsRow, GoalsManager};
use jiff_sqlx::Date;
use serenity::all::{CreateCommand, UserId};
use sqlx::{PgPool, Postgres};
use zayden_core::as_i64;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct DailyTable;

#[async_trait]
impl DailyManager<Postgres> for DailyTable {
    async fn daily_row(pool: &PgPool, id: UserId) -> sqlx::Result<Option<DailyRow>> {
        sqlx::query_file_as!(
            DailyRow,
            "sql/gambling/DailyManager/daily_row.sql",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    async fn goal_rows(
        pool: &PgPool,
        id: UserId,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>> {
        sqlx::query_file_as!(
            GamblingGoalsRow,
            "sql/gambling/DailyManager/goal_rows.sql",
            as_i64(id.get())
        )
        .fetch_all(pool)
        .await
    }

    async fn claim_daily(
        pool: &PgPool,
        id: UserId,
        amount: i64,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query!(
            "INSERT INTO gambling (user_id, coins, daily)
            VALUES ($1, $2, (now() AT TIME ZONE 'UTC')::date)
            ON CONFLICT (user_id) DO UPDATE SET
                coins = gambling.coins + $2,
                daily = (now() AT TIME ZONE 'UTC')::date
            WHERE gambling.daily <> (now() AT TIME ZONE 'UTC')::date",
            as_i64(id.get()),
            amount,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }
}

#[async_trait]
impl GoalsManager<Postgres> for DailyTable {
    async fn row(_pool: &PgPool, _id: UserId) -> sqlx::Result<Option<GoalsRow>> {
        Ok(None)
    }

    async fn full_rows(
        _pool: &PgPool,
        _id: UserId,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>> {
        Ok(Vec::new())
    }

    #[expect(
        trivial_casts,
        reason = "sqlx requires explicit type for jiff_sqlx DATE[] mapping"
    )]
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
        let mut days: Vec<Date> = Vec::with_capacity(num_rows);
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
            r#"INSERT INTO gambling_goals (user_id, goal_id, day, progress, target)
            SELECT * FROM UNNEST($1::bigint[], $2::text[], $3::date[], $4::bigint[], $5::bigint[])
            RETURNING user_id, goal_id, day as "day: jiff_sqlx::Date", progress, target;"#,
            &user_ids,
            &goal_ids,
            &days as &[Date],
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
impl ModuleCommand for Daily {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("daily")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_daily()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::daily::<BotState, Postgres, DailyTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
