use std::collections::HashMap;
use std::fs;

use serenity::all::{
    AutocompleteChoice,
    AutocompleteOption,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateAutocompleteResponse,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    CreateEmbedFooter,
    CreateInteractionResponse,
    EditInteractionResponse,
    Http,
    ResolvedOption,
};
use zayden_core::sole_option;

use crate::compendium::PerkInfo;
use crate::{DestinyError, Result, compendium};

pub struct Perk;

impl Perk {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        api_key: &str,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let perk: &str = sole_option(&mut options)?;

        let perks_json = match fs::read_to_string("perks.json") {
            Ok(s) => s,
            Err(_) => {
                compendium::update(api_key).await?;
                fs::read_to_string("perks.json")?
            },
        };
        let mut perks: HashMap<String, PerkInfo> =
            serde_json::from_str(&perks_json)?;
        let Some(perk) = perks.remove(&perk.to_lowercase()) else {
            return Err(DestinyError::PerkNotFound(perk.to_string()));
        };

        let embed = CreateEmbed::new()
            .title(perk.name)
            .description(perk.description)
            .footer(CreateEmbedFooter::new("From 'Destiny Data Compendium'"));

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("perk").description("Perk information").add_option(
            CreateCommandOption::new(CommandOptionType::String, "perk", "Perk name")
                .required(true)
                .set_autocomplete(true),
        )
    }

    pub async fn autocomplete(
        http: &Http,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        api_key: &str,
    ) -> Result<()> {
        let perks_json = match fs::read_to_string("perks.json") {
            Ok(s) => s,
            Err(_) => {
                compendium::update(api_key).await?;
                fs::read_to_string("perks.json")?
            },
        };
        let perks: HashMap<String, PerkInfo> = serde_json::from_str(&perks_json)?;

        let perks = perks
            .into_iter()
            .filter(|(name, _)| name.contains(&option.value.to_lowercase()))
            .map(|(_, perk)| AutocompleteChoice::from(perk.name))
            .take(25)
            .collect::<Vec<_>>();

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(perks),
                ),
            )
            .await?;

        Ok(())
    }
}
