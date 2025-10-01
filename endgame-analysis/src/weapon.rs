use std::fs;

use destiny2_core::BungieClientData;
use serenity::all::{
    AutocompleteChoice, AutocompleteOption, CommandInteraction, CommandOptionType, Context,
    CreateAutocompleteResponse, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    EditInteractionResponse, ResolvedValue,
};
use tokio::sync::RwLock;
use zayden_core::parse_options;

use crate::{Error, Result};

use super::endgame_analysis::EndgameAnalysisSheet;
use super::endgame_analysis::weapon::Weapon;

pub struct WeaponCommand;

impl WeaponCommand {
    pub async fn run<Data: BungieClientData>(
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let options = interaction.data.options();
        let options = parse_options(options);

        let name = match options.get("name") {
            Some(ResolvedValue::String(name)) => name,
            _ => unreachable!("Name is required"),
        };

        let weapons: Vec<Weapon> = if let Ok(w) = fs::read_to_string("weapons.json") {
            serde_json::from_str(&w).unwrap()
        } else {
            let item_manifest = {
                let data_lock = ctx.data::<RwLock<Data>>();
                let data = data_lock.read().await;
                let client = data.bungie_client();
                let manifest = client.destiny_manifest().await.unwrap();
                client
                    .destiny_inventory_item_definition(&manifest, "en")
                    .await
                    .unwrap()
            };

            EndgameAnalysisSheet::update(&item_manifest).await?;
            let w = fs::read_to_string("weapons.json").unwrap();
            serde_json::from_str(&w).unwrap()
        };

        let weapon = weapons
            .iter()
            .find(|&w| w.name().to_lowercase() == name.to_lowercase())
            .ok_or_else(|| Error::WeaponNotFound(name.to_string()))?;

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().embed(weapon.into()),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("weapon")
            .description("Get a weapon from Destiny 2")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "The name of the weapon",
                )
                .required(true)
                .set_autocomplete(true),
            )
    }

    pub async fn autocomplete<Data: BungieClientData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
    ) -> Result<()> {
        let weapons = match std::fs::read_to_string("weapons.json") {
            Ok(weapons) => weapons,
            Err(_) => {
                let item_manifest = {
                    let data_lock = ctx.data::<RwLock<Data>>();
                    let data = data_lock.read().await;
                    let client = data.bungie_client();
                    let manifest = client.destiny_manifest().await.unwrap();
                    client
                        .destiny_inventory_item_definition(&manifest, "en")
                        .await
                        .unwrap()
                };

                EndgameAnalysisSheet::update(&item_manifest).await?;
                std::fs::read_to_string("weapons.json").unwrap()
            }
        };
        let weapons: Vec<Weapon> = serde_json::from_str(&weapons).unwrap();
        let weapons = weapons
            .into_iter()
            .filter(|w| {
                w.name()
                    .to_lowercase()
                    .contains(&option.value.to_lowercase())
            })
            .map(AutocompleteChoice::from)
            .take(25)
            .collect::<Vec<_>>();

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(weapons),
                ),
            )
            .await?;

        Ok(())
    }
}
