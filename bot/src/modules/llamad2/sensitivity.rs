use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{Error, Result};

pub struct Sensitivity;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Sensitivity {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        llamad2::Sensitivity::run(ctx, interaction).await;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(llamad2::Sensitivity::register())
    }
}
