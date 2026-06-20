use std::borrow::Cow;

use async_trait::async_trait;
use futures::TryStreamExt;
use gambling::Commands;
use gambling::games::higherlower::HigherLowerManager;
use serenity::all::{CreateCommand, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, Postgres};
use tracing::debug;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;
use zayden_core::{as_u64, message_metadata};

use super::{GameTable, GoalsTable, StatsTable};
use crate::BotState;
use crate::bindings::gambling::GamblingTable;

pub struct HigherLowerTable;

#[async_trait]
impl HigherLowerManager<Postgres> for HigherLowerTable {
    async fn winners(conn: &mut PgConnection) -> sqlx::Result<Vec<UserId>> {
        sqlx::query_file_scalar!("sql/gambling/HigherLowerManager/winners.sql")
            .fetch(conn)
            .map_ok(|id| UserId::new(as_u64(id)))
            .try_collect()
            .await
    }

    async fn reset(conn: &mut PgConnection) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file_scalar!("sql/gambling/HigherLowerManager/reset.sql")
            .execute(conn)
            .await
    }
}

pub struct HigherLower;

#[async_trait]
impl ModuleCommand for HigherLower {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("higherorlower")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_higher_lower()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::higher_lower::<BotState>(cx.ctx, cx.interaction).await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for HigherLower {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("hol"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let metadata = message_metadata(&cx.interaction.message)?;

        if cx.interaction.user != metadata.user {
            debug!(
                user_id = %cx.interaction.user.id,
                owner_id = %metadata.user.id,
                "user does not own this higher/lower message; ignoring"
            );
            return Ok(());
        }

        gambling::components::HigherLower::run_components::<
            BotState,
            Postgres,
            GamblingTable,
            GameTable,
            GoalsTable,
            StatsTable,
        >(cx.ctx, cx.interaction, &cx.app.db)
        .await?;
        Ok(())
    }
}
