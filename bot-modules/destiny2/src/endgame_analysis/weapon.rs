use std::fs;

use bungie_api::BungieClient;
use serenity::all::{
    AutocompleteChoice,
    AutocompleteOption,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateAutocompleteResponse,
    CreateCommandOption,
    CreateInteractionResponse,
    EditInteractionResponse,
    ResolvedOption,
};
use zayden_core::sole_option;

use crate::endgame_analysis::sheet::EndgameAnalysisSheet;
use crate::endgame_analysis::sheet::weapon::Weapon;
use crate::endgame_analysis::{EndgameAnalysisError, Result};

pub struct WeaponCommand;

impl WeaponCommand {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        client: &BungieClient,
        api_key: &str,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let name: &str = sole_option(&mut options)?;

        let weapons: Vec<Weapon> = match fs::read_to_string("weapons.json") {
            Ok(w) => serde_json::from_str(&w)?,
            Err(_) => {
                let manifest = client.destiny_manifest().await?;
                let item_manifest = client
                    .destiny_inventory_item_definition(&manifest, "en")
                    .await?;

                EndgameAnalysisSheet::update(&item_manifest, api_key).await?;
                let w = fs::read_to_string("weapons.json")?;
                serde_json::from_str(&w)?
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

    pub fn register<'a>() -> CreateCommandOption<'a> {
        CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "weapon",
            "Get a weapon from Destiny 2",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the weapon",
            )
            .required(true)
            .set_autocomplete(true),
        )
    }

    pub async fn autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        client: &BungieClient,
        api_key: &str,
    ) -> Result<()> {
        let weapons: Vec<Weapon> = match fs::read_to_string("weapons.json") {
            Ok(weapons) => serde_json::from_str(&weapons)?,
            Err(_) => {
                let manifest = client.destiny_manifest().await?;
                let item_manifest = client
                    .destiny_inventory_item_definition(&manifest, "en")
                    .await?;

                EndgameAnalysisSheet::update(&item_manifest, api_key).await?;
                let weapons = fs::read_to_string("weapons.json")?;
                serde_json::from_str(&weapons)?
            },
        };

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
