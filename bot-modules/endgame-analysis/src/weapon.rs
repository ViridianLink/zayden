use std::fs;

use destiny2_core::BungieClientData;
use serenity::all::{
    AutocompleteChoice,
    AutocompleteOption,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateAutocompleteResponse,
    CreateCommand,
    CreateCommandOption,
    CreateInteractionResponse,
    EditInteractionResponse,
    ResolvedValue,
};
use tokio::sync::RwLock;
use zayden_core::parse_options;

use super::endgame_analysis::EndgameAnalysisSheet;
use super::endgame_analysis::weapon::Weapon;
use crate::{EndgameAnalysisError, Result};

pub struct WeaponCommand;

impl WeaponCommand {
    pub async fn run<Data: BungieClientData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        api_key: &str,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let options = interaction.data.options();
        let mut options = parse_options(options);

        let Some(ResolvedValue::String(name)) = options.remove("name") else {
            return Ok(());
        };

        let weapons: Vec<Weapon> = match fs::read_to_string("weapons.json") {
            Ok(w) => serde_json::from_str(&w).expect("valid JSON"),
            Err(_) => {
                let client = {
                    let data_lock = ctx.data::<RwLock<Data>>();
                    let data = data_lock.read().await;
                    data.bungie_client()
                };

                let manifest = client.destiny_manifest().await?;
                let item_manifest = client
                    .destiny_inventory_item_definition(&manifest, "en")
                    .await
                    .expect("data invariant");

                EndgameAnalysisSheet::update(&item_manifest, api_key).await?;
                let w = fs::read_to_string("weapons.json")
                    .expect("weapons.json readable");
                serde_json::from_str(&w).expect("valid JSON")
            },
        };

        let weapon = weapons
            .iter()
            .find(|&w| w.name().to_lowercase() == name.to_lowercase())
            .ok_or_else(|| EndgameAnalysisError::WeaponNotFound(name.to_string()))?;

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().embed(weapon.into()),
            )
            .await?;

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
        api_key: &str,
    ) -> Result<()> {
        let weapons = match fs::read_to_string("weapons.json") {
            Ok(weapons) => weapons,
            Err(_) => {
                let client = {
                    let data_lock = ctx.data::<RwLock<Data>>();
                    let data = data_lock.read().await;
                    data.bungie_client()
                };

                let manifest = client.destiny_manifest().await?;
                let item_manifest = client
                    .destiny_inventory_item_definition(&manifest, "en")
                    .await
                    .expect("data invariant");

                EndgameAnalysisSheet::update(&item_manifest, api_key).await?;
                fs::read_to_string("weapons.json").expect("weapons.json readable")
            },
        };
        let weapons: Vec<Weapon> =
            serde_json::from_str(&weapons).expect("valid weapons JSON");
        let weapons = weapons
            .into_iter()
            .filter(|w| {
                w.name().to_lowercase().contains(&option.value.to_lowercase())
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
