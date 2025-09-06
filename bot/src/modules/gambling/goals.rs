use async_trait::async_trait;
use chrono::NaiveDate;
use gambling::commands::goals::GoalsRow;
use gambling::{Commands, GamblingGoalsRow, GoalsManager};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{CtxData, Error, Result};

pub struct GoalsTable;

#[async_trait]
impl GoalsManager<Postgres> for GoalsTable {
    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<GoalsRow>> {
        let id = id.into();

        sqlx::query_as!(
            GoalsRow,
            "SELECT
                g.coins,
                g.gems,

                COALESCE(l.level, 0) AS level,
                
                COALESCE(m.prestige, 0) AS prestige

                FROM gambling g
                LEFT JOIN levels l ON g.id = l.id
                LEFT JOIN gambling_mine m on g.id = m.id
                WHERE g.id = $1;",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn full_rows(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>> {
        let id = id.into();

        sqlx::query_as!(
            GamblingGoalsRow,
            "SELECT user_id, goal_id, day, progress, target FROM gambling_goals WHERE user_id = $1",
            id.get() as i64
        )
        .fetch_all(pool)
        .await
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

pub struct Goals;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Goals {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::goals::<CtxData, Postgres, GoalsTable>(ctx, interaction, pool).await?;
        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_goals())
    }
}
