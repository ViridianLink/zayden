use async_trait::async_trait;
use endgame_analysis::{DimWishlistCommand, TierListCommand, WeaponCommand};
use serenity::all::{
    AutocompleteOption, CommandInteraction, Context, CreateCommand, ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{Autocomplete, SlashCommand};

use crate::{Error, Result};

use super::{DestinyPerkTable, DestinyWeaponTable};

pub struct DimWishlist;

#[async_trait]
impl SlashCommand<Error, Postgres> for DimWishlist {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        DimWishlistCommand::run::<Postgres, DestinyWeaponTable, DestinyPerkTable>(
            &ctx.http,
            interaction,
            options,
            pool,
        )
        .await;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        Ok(DimWishlistCommand::register())
    }
}

pub struct TierList;

#[async_trait]
impl SlashCommand<Error, Postgres> for TierList {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        TierListCommand::run::<Postgres, DestinyWeaponTable>(&ctx.http, interaction, options, pool)
            .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        Ok(TierListCommand::register())
    }
}

#[async_trait]
impl Autocomplete<Error, Postgres> for TierList {
    async fn autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        pool: &PgPool,
    ) -> Result<()> {
        TierListCommand::autocomplete::<Postgres, DestinyWeaponTable>(
            &ctx.http,
            interaction,
            option,
            pool,
        )
        .await?;

        Ok(())
    }
}

pub struct Weapon;

#[async_trait]
impl SlashCommand<Error, Postgres> for Weapon {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        WeaponCommand::run::<Postgres, DestinyWeaponTable>(&ctx.http, interaction, pool).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        Ok(WeaponCommand::register())
    }
}

#[async_trait]
impl Autocomplete<Error, Postgres> for Weapon {
    async fn autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        pool: &PgPool,
    ) -> Result<()> {
        WeaponCommand::autocomplete::<Postgres, DestinyWeaponTable>(
            &ctx.http,
            interaction,
            option,
            pool,
        )
        .await?;

        Ok(())
    }
}
