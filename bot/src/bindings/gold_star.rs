use std::borrow::Cow;

use async_trait::async_trait;
use gold_star::{GiveStar, GoldStarManager, GoldStarRow, Stars};
use serenity::all::CreateCommand;
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
        .await
        .map_err(HandlerError::from_respond)
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
        .await
        .map_err(HandlerError::from_respond)
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

    async fn save_row(pool: &PgPool, row: &GoldStarRow) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO gold_stars \
             (id, number_of_stars, given_stars, received_stars, last_free_star) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (id) DO UPDATE SET \
             number_of_stars = EXCLUDED.number_of_stars, \
             given_stars = EXCLUDED.given_stars, \
             received_stars = EXCLUDED.received_stars, \
             last_free_star = EXCLUDED.last_free_star",
        )
        .bind(row.id)
        .bind(row.number_of_stars)
        .bind(row.given_stars)
        .bind(row.received_stars)
        .bind(row.last_free_star)
        .execute(pool)
        .await?;
        Ok(())
    }
}
