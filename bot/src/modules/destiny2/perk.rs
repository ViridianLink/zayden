use async_trait::async_trait;
use serenity::all::{
    AutocompleteOption, CommandInteraction, Context, CreateCommand, ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{ApplicationCommand, Autocomplete};

use crate::{Error, Result};

pub struct Perk;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Perk {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        destiny2::slash_commands::perk::Perk::run(ctx, interaction, options)
            .await
            .unwrap();

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(destiny2::slash_commands::perk::Perk::register())
    }
}

#[async_trait]
impl Autocomplete<Error, Postgres> for Perk {
    async fn autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        _pool: &PgPool,
    ) -> Result<()> {
        destiny2::slash_commands::perk::Perk::autocomplete(&ctx.http, interaction, option).await;

        Ok(())
    }
}
