use async_trait::async_trait;
use endgame_analysis::{DimWishlistCommand, TierListCommand, WeaponCommand};
use serenity::all::{
    AutocompleteOption, CommandInteraction, Context, CreateCommand, ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{ApplicationCommand, Autocomplete};

use crate::{CtxData, Error, Result};

pub struct DimWishlist;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for DimWishlist {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        DimWishlistCommand::run::<CtxData>(ctx, interaction, options).await;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(DimWishlistCommand::register())
    }
}

pub struct TierList;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for TierList {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        TierListCommand::run::<CtxData>(ctx, interaction, options).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(TierListCommand::register())
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
        TierListCommand::autocomplete::<CtxData>(ctx, interaction, option).await?;

        Ok(())
    }
}

pub struct Weapon;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Weapon {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        WeaponCommand::run::<CtxData>(ctx, interaction).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(WeaponCommand::register())
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
        WeaponCommand::autocomplete::<CtxData>(ctx, interaction, option).await?;

        Ok(())
    }
}
