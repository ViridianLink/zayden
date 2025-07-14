use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use zayden_core::SlashCommand;

use crate::{Error, Result};

use super::ReactionRolesTable;

pub struct ReactionRoleCommand;

#[async_trait]
impl SlashCommand<Error, Postgres> for ReactionRoleCommand {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        reaction_roles::ReactionRoleCommand::run::<Postgres, ReactionRolesTable>(
            &ctx.http,
            interaction,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        Ok(reaction_roles::ReactionRoleCommand::register())
    }
}
