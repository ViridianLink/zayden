use std::borrow::Cow;

use async_trait::async_trait;
use gold_star::{GiveStar, GoldStarError, GoldStarManager, GoldStarRow, Stars};
use serenity::all::{CreateCommand, UserId};
use sqlx::{PgPool, Postgres};
use zayden_core::{HandlerError, InvocationCtx, ModuleCommand};

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(GiveStarCmd);
    builder.add_command(StarsCmd);
}

pub struct GiveStarCmd;

#[async_trait]
impl ModuleCommand for GiveStarCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("give_star")
    }

    fn definition(&self) -> CreateCommand<'static> {
        GiveStar::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        GiveStar::run::<Postgres, GoldStarTable>(
            &cx.ctx.http,
            cx.interaction,
            cx.interaction.data.options(),
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct StarsCmd;

#[async_trait]
impl ModuleCommand for StarsCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("stars")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Stars::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Stars::run::<Postgres, GoldStarTable>(
            &cx.ctx.http,
            cx.interaction,
            cx.interaction.data.options(),
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct GoldStarTable;

#[async_trait]
impl GoldStarManager<Postgres> for GoldStarTable {
    async fn get_row(
        pool: &PgPool,
        user_id: impl Into<i64> + Send,
    ) -> sqlx::Result<Option<GoldStarRow>> {
        let user_id = user_id.into();
        sqlx::query_as::<_, GoldStarRow>(
            "SELECT id, number_of_stars, given_stars, received_stars, last_free_star \
             FROM gold_stars WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    async fn give_star(
        pool: &PgPool,
        author_id: UserId,
        target_id: UserId,
    ) -> Result<i32, GoldStarError> {
        let author_id = author_id.get().cast_signed();
        let target_id = target_id.get().cast_signed();

        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO gold_stars (id, last_free_star) VALUES ($1, to_timestamp(0)) \
             ON CONFLICT (id) DO NOTHING",
            author_id
        )
        .execute(&mut *tx)
        .await?;

        let author = sqlx::query!(
            r#"SELECT
                number_of_stars,
                (last_free_star + INTERVAL '24 hours') <= now() AS "free_star!",
                EXTRACT(EPOCH FROM last_free_star + INTERVAL '24 hours')::bigint AS "next_free_star!"
               FROM gold_stars
               WHERE id = $1
               FOR UPDATE"#,
            author_id
        )
        .fetch_one(&mut *tx)
        .await?;

        if author.number_of_stars < 1 && !author.free_star {
            return Err(GoldStarError::NoStars(author.next_free_star));
        }

        if author.free_star {
            sqlx::query!(
                "UPDATE gold_stars SET given_stars = given_stars + 1, last_free_star = now() \
                 WHERE id = $1",
                author_id
            )
            .execute(&mut *tx)
            .await?;
        } else {
            let debit = sqlx::query!(
                "UPDATE gold_stars SET number_of_stars = number_of_stars - 1, given_stars = given_stars + 1 \
                 WHERE id = $1 AND number_of_stars >= 1",
                author_id
            )
            .execute(&mut *tx)
            .await?;

            if debit.rows_affected() != 1 {
                return Err(GoldStarError::NoStars(author.next_free_star));
            }
        }

        let target = sqlx::query!(
            r#"INSERT INTO gold_stars (id, number_of_stars, received_stars, last_free_star)
               VALUES ($1, 1, 1, to_timestamp(0))
               ON CONFLICT (id) DO UPDATE SET
                   number_of_stars = gold_stars.number_of_stars + 1,
                   received_stars = gold_stars.received_stars + 1
               RETURNING number_of_stars"#,
            target_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(target.number_of_stars)
    }
}
