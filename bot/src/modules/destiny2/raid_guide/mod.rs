use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{Error, Result};

pub struct RaidGuide;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for RaidGuide {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        destiny2::raid_guides::RaidGuide::<0>::run(&ctx.http, interaction).await;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        destiny2::raid_guides::RaidGuide::<0>::register()
    }
}
