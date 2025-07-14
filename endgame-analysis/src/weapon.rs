use std::fs;

use serenity::all::{
    AutocompleteChoice, AutocompleteOption, CommandInteraction, CommandOptionType,
    CreateAutocompleteResponse, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    EditInteractionResponse, Http, ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::{DestinyWeaponManager, Error, Result};

use super::endgame_analysis::EndgameAnalysisSheet;
use super::endgame_analysis::weapon::Weapon;

pub struct WeaponCommand;

impl WeaponCommand {
    pub async fn run<Db: Database, Manager: DestinyWeaponManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(http).await.unwrap();

        let options = interaction.data.options();
        let options = parse_options(options);

        let name = match options.get("name") {
            Some(ResolvedValue::String(name)) => name,
            _ => unreachable!("Name is required"),
        };

        let weapons: Vec<Weapon> = if let Ok(w) = fs::read_to_string("weapons.json") {
            serde_json::from_str(&w).unwrap()
        } else {
            EndgameAnalysisSheet::update::<Db, Manager>(pool).await?;
            let w = fs::read_to_string("weapons.json").unwrap();
            serde_json::from_str(&w).unwrap()
        };

        let weapon = weapons
            .iter()
            .find(|&w| w.name().to_lowercase() == name.to_lowercase())
            .ok_or_else(|| Error::WeaponNotFound(name.to_string()))?;

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(weapon.into()))
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

    pub async fn autocomplete<Db: Database, Manager: DestinyWeaponManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let weapons = match std::fs::read_to_string("weapons.json") {
            Ok(weapons) => weapons,
            Err(_) => {
                EndgameAnalysisSheet::update::<Db, Manager>(pool).await?;
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
                http,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(weapons),
                ),
            )
            .await?;

        Ok(())
    }
}
