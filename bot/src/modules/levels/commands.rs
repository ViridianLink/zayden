use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{CtxData, Error, Result};

use super::LevelsTable;

pub struct Levels;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Levels {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        levels::Levels::run::<CtxData, Postgres, LevelsTable>(ctx, interaction, pool).await;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(levels::Levels::register())
    }
}

pub struct Rank;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Rank {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        levels::Rank::rank::<Postgres, LevelsTable>(&ctx.http, interaction, options, pool).await;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(levels::Rank::register())
    }
}

pub struct Xp;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Xp {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        levels::Xp::xp::<Postgres, LevelsTable>(&ctx.http, interaction, options, pool).await;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(levels::Xp::register())
    }
}
