use bungie_api::BungieClient;
use serenity::all::{
    AutocompleteOption,
    CommandInteraction,
    Context,
    CreateCommand,
};
use sqlx::PgPool;
use zayden_core::error::HandlerError;
use zayden_core::{CoreError, EmojiCacheData, parse_subcommand};

use crate::endgame_analysis::{DimWishlistCommand, TierListCommand, WeaponCommand};
use crate::loadouts::Loadout;
use crate::raid_guides::RaidGuide;
use crate::slash_commands::perk::Perk;

pub struct Command;

impl Command {
    pub fn register() -> CreateCommand<'static> {
        CreateCommand::new("destiny2")
            .description(
                "Destiny 2: perks, weapons, tier lists, wishlists, builds, and raid guides",
            )
            .add_option(Perk::register())
            .add_option(WeaponCommand::register())
            .add_option(TierListCommand::register())
            .add_option(DimWishlistCommand::register())
            .add_option(Loadout::register())
            .add_option(RaidGuide::<0>::register())
    }

    pub async fn run<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
        client: &BungieClient,
        api_key: &str,
        parent_token: &str,
    ) -> Result<(), HandlerError> {
        let (name, sub_options) = parse_subcommand(interaction.data.options())?;

        match name {
            "perk" => {
                Perk::run(ctx, interaction, sub_options.into_vec(), pool, api_key)
                    .await?;
            },
            "weapon" => {
                WeaponCommand::run(
                    ctx,
                    interaction,
                    sub_options.into_vec(),
                    pool,
                    client,
                    api_key,
                )
                .await?;
            },
            "tierlist" => {
                TierListCommand::run(
                    ctx,
                    interaction,
                    sub_options.into_vec(),
                    pool,
                    client,
                    api_key,
                )
                .await?;
            },
            "dimwishlist" => {
                DimWishlistCommand::run(
                    ctx,
                    interaction,
                    sub_options.into_vec(),
                    pool,
                    client,
                    api_key,
                )
                .await?;
            },
            "builds" => {
                Loadout::run::<Data>(
                    ctx,
                    interaction,
                    sub_options.into_vec(),
                    parent_token,
                )
                .await?;
            },
            "raidguide" => {
                RaidGuide::<0>::run(&ctx.http, interaction).await?;
            },
            other => {
                return Err(HandlerError::from_respond(CoreError::invalid_option(
                    format!("unknown subcommand: {other}"),
                )));
            },
        }

        Ok(())
    }

    pub async fn autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        pool: &PgPool,
        client: &BungieClient,
        api_key: &str,
    ) -> Result<(), HandlerError> {
        let (name, _) = parse_subcommand(interaction.data.options())?;

        match name {
            "perk" => {
                Perk::autocomplete(&ctx.http, interaction, option, pool, api_key)
                    .await?;
            },
            "weapon" => {
                WeaponCommand::autocomplete(
                    ctx,
                    interaction,
                    option,
                    pool,
                    client,
                    api_key,
                )
                .await?;
            },
            "tierlist" => {
                TierListCommand::autocomplete(
                    ctx,
                    interaction,
                    option,
                    pool,
                    client,
                    api_key,
                )
                .await?;
            },
            _ => {},
        }

        Ok(())
    }
}
