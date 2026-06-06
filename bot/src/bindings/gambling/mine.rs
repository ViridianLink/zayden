use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::mine::{MineManager, MineRow};
use serenity::all::{CreateCommand, UserId};
use sqlx::{PgPool, Postgres};
use zayden_core::as_i64;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct MineTable;

#[async_trait]
impl MineManager<Postgres> for MineTable {
    async fn row(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<MineRow>> {
        let id = id.into();

        sqlx::query_as!(
            MineRow,
            "SELECT miners, mines, land, countries, continents, planets, solar_systems, galaxies, universes, prestige FROM gambling_mine WHERE user_id = $1",
            as_i64(id.get())
        ).fetch_optional(pool).await
    }
}

pub struct Mine;

#[async_trait]
impl ModuleCommand for Mine {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("mine")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_mine()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::mine::<BotState, Postgres, MineTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
