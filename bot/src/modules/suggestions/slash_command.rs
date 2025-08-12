use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{Error, Result, sqlx_lib::GuildTable};

pub struct FetchSuggestions;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for FetchSuggestions {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        suggestions::FetchSuggestions::run::<Postgres, GuildTable>(
            &ctx.http,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(suggestions::FetchSuggestions::register())
    }
}
