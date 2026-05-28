use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{ApplicationCommand, Component};

use crate::{BotState, Error, Result};

use super::LevelsTable;

pub struct Levels;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Levels {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        levels::Levels::run::<BotState, Postgres, LevelsTable>(ctx, interaction, pool).await;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        levels::Levels::register()
    }
}

#[async_trait]
impl Component<Error, Postgres> for Levels {
    async fn run(ctx: &Context, interaction: &ComponentInteraction, pool: &PgPool) -> Result<()> {
        levels::Levels::run_components::<BotState, Postgres, LevelsTable>(ctx, interaction, pool)
            .await;

        Ok(())
    }
}

pub struct Rank;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Rank {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        levels::Rank::rank::<Postgres, LevelsTable>(&ctx.http, interaction, options, pool).await;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        levels::Rank::register()
    }
}

pub struct Xp;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Xp {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        levels::Xp::xp::<Postgres, LevelsTable>(&ctx.http, interaction, options, pool).await;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        levels::Xp::register()
    }
}
