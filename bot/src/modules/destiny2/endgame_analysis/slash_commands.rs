use async_trait::async_trait;
use endgame_analysis::{DimWishlistCommand, TierListCommand, WeaponCommand};
use serenity::all::{
    AutocompleteOption, CommandInteraction, Context, CreateCommand, ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{ApplicationCommand, Autocomplete};

use crate::{BotState, Error, Result};

pub struct DimWishlist;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for DimWishlist {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        DimWishlistCommand::run::<BotState>(ctx, interaction, options).await;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        DimWishlistCommand::register()
    }
}

pub struct TierList;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for TierList {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        TierListCommand::run::<BotState>(ctx, interaction, options).await?;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        TierListCommand::register()
    }
}

#[async_trait]
impl Autocomplete<Error, Postgres> for TierList {
    async fn autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        _pool: &PgPool,
    ) -> Result<()> {
        TierListCommand::autocomplete::<BotState>(ctx, interaction, option).await?;

        Ok(())
    }
}

pub struct Weapon;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Weapon {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        WeaponCommand::run::<BotState>(ctx, interaction).await?;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        WeaponCommand::register()
    }
}

#[async_trait]
impl Autocomplete<Error, Postgres> for Weapon {
    async fn autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        _pool: &PgPool,
    ) -> Result<()> {
        WeaponCommand::autocomplete::<BotState>(ctx, interaction, option).await?;

        Ok(())
    }
}
