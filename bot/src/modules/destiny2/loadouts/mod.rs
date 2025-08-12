use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{CtxData, Error, Result};

pub struct Loadout;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Loadout {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        destiny2::loadouts::Loadout::run::<CtxData>(ctx, interaction, options).await;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(destiny2::loadouts::Loadout::register())
    }
}
