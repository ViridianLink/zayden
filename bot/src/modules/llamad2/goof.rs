use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{Error, Result};

pub struct Goof;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Goof {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        llamad2::Goof::run(ctx, interaction).await;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        llamad2::Goof::register()
    }
}
