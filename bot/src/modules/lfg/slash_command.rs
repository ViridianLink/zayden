use async_trait::async_trait;
use serenity::all::{
    AutocompleteOption, CommandInteraction, Context, CreateCommand, ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{ApplicationCommand, Autocomplete};

use crate::{Error, Result};

use super::{PostTable, UsersTable};

pub struct Lfg;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Lfg {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        lfg::Command::lfg::<Postgres, UsersTable, PostTable>(&ctx.http, interaction, options, pool)
            .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(lfg::Command::register())
    }
}

#[async_trait]
impl Autocomplete<Error, Postgres> for Lfg {
    async fn autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        _pool: &PgPool,
    ) -> Result<()> {
        lfg::Command::autocomplete(&ctx.http, interaction, option).await?;

        Ok(())
    }
}
