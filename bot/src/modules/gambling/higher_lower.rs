use async_trait::async_trait;
use futures::TryStreamExt;
use gambling::Commands;
use gambling::games::higherlower::HigherLowerManager;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, PgPool, Postgres};
use zayden_core::SlashCommand;

use crate::{CtxData, Error, Result};

use super::goals::GoalsTable;
use super::{GameTable, StatsTable};

pub struct HigherLowerTable;

#[async_trait]
impl HigherLowerManager<Postgres> for HigherLowerTable {
    async fn winners(conn: &mut PgConnection) -> sqlx::Result<Vec<UserId>> {
        sqlx::query_file_scalar!("sql/gambling/HigherLowerManager/winners.sql")
            .fetch(conn)
            .map_ok(|id| UserId::new(id as u64))
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
impl SlashCommand<Error, Postgres> for HigherLower {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::higher_lower::<CtxData, Postgres, GoalsTable, GameTable, StatsTable>(
            ctx,
            interaction,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        Ok(Commands::register_higher_lower())
    }
}
