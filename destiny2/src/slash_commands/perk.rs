use std::collections::HashMap;

use serenity::all::{
    AutocompleteChoice, AutocompleteOption, CommandInteraction, CommandOptionType, Context,
    CreateAutocompleteResponse, CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, EditInteractionResponse, Http, ResolvedOption, ResolvedValue,
};

use crate::compendium;
use crate::compendium::PerkInfo;

pub struct Perk;

impl Perk {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
    ) -> Result<(), ()> {
        interaction.defer(&ctx.http).await.unwrap();

        let ResolvedValue::String(perk) = options[0].value else {
            unreachable!("Option must be string")
        };

        let perks = match std::fs::read_to_string("perks.json") {
            Ok(perks) => perks,
            Err(_) => {
                compendium::update().await;
                std::fs::read_to_string("perks.json").unwrap()
            }
        };
        let mut perks: HashMap<String, PerkInfo> = serde_json::from_str(&perks).unwrap();
        let perk = match perks.remove(&perk.to_lowercase()) {
            Some(perk) => perk,
            None => {
                interaction
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content(format!("No perk found for: {}", perk)),
                    )
                    .await
                    .unwrap();

                return Ok(());
            }
        };

        let embed = CreateEmbed::new()
            .title(perk.name)
            .description(perk.description)
            .footer(CreateEmbedFooter::new("From 'Destiny Data Compendium'"));

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("perk")
            .description("Perk information")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "perk", "Perk name")
                    .required(true)
                    .set_autocomplete(true),
            )
    }

    pub async fn autocomplete(
        http: &Http,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
    ) {
        let perks = match std::fs::read_to_string("perks.json") {
            Ok(perks) => perks,
            Err(_) => {
                compendium::update().await;
                std::fs::read_to_string("perks.json").unwrap()
            }
        };
        let perks: HashMap<String, PerkInfo> = serde_json::from_str(&perks).unwrap();
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
            .await
            .unwrap();
    }
}
