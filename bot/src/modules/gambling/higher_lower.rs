use async_trait::async_trait;
use futures::TryStreamExt;
use gambling::Commands;
use gambling::games::higherlower::HigherLowerManager;
use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, MessageInteractionMetadata,
    ResolvedOption, UserId,
};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, PgPool, Postgres};
use zayden_core::{ApplicationCommand, Component};

use crate::modules::gambling::GamblingTable;
use crate::{CtxData, Error, Result};

use super::{GameTable, GoalsTable, StatsTable};

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
impl ApplicationCommand<Error, Postgres> for HigherLower {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        Commands::higher_lower::<CtxData>(ctx, interaction).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_higher_lower())
    }
}

#[async_trait]
impl Component<Error, Postgres> for HigherLower {
    async fn run(ctx: &Context, interaction: &ComponentInteraction, pool: &PgPool) -> Result<()> {
        let Some(MessageInteractionMetadata::Command(metadata)) =
            interaction.message.interaction_metadata.as_deref()
        else {
            unreachable!("Message must be created from an command")
        };

        if interaction.user != metadata.user {
            return Ok(());
        };

        gambling::components::HigherLower::run_components::<
            CtxData,
            Postgres,
            GamblingTable,
            GameTable,
            GoalsTable,
            StatsTable,
        >(ctx, interaction, pool)
        .await?;

        Ok(())
    }
}
