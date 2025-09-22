use std::collections::{HashMap, HashSet};
use std::fs;

use destiny2_core::BungieClientData;
use serenity::all::{
    AutocompleteChoice, AutocompleteOption, CommandInteraction, CommandOptionType, Context,
    CreateAutocompleteResponse, CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use tokio::sync::RwLock;
use zayden_core::parse_options;

use crate::Result;
use crate::endgame_analysis::EndgameAnalysisSheet;
use crate::endgame_analysis::weapon::Weapon;

use super::endgame_analysis::tier::TIERS;

pub struct TierListCommand;

impl TierListCommand {
    pub async fn run<Data: BungieClientData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let mut options = parse_options(options);

        let Some(ResolvedValue::String(archetype)) = options.remove("archetype") else {
            unreachable!("Archetype is required");
        };

        let count = options.get("count").map(|c| match c {
            ResolvedValue::Integer(c) => *c as usize,
            _ => unreachable!("Count must be an integer"),
        });

        let tiers = match options.get("tier") {
            Some(ResolvedValue::String(tier)) => {
                let tier = tier.parse().unwrap();
                let index = TIERS.iter().copied().position(|t| t == tier).unwrap();
                &TIERS[..=index]
            }
            _ => &TIERS,
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

        let init_map = tiers
            .iter()
            .copied()
            .map(|t| (t, Vec::new()))
            .collect::<HashMap<_, _>>();

        let weapons = weapons
            .into_iter()
            .filter(|w| w.archetype() == archetype)
            .filter(|w| tiers.contains(&w.tier.tier))
            .take(count.unwrap_or(usize::MAX))
            .fold(init_map, |mut map, w| {
                map.get_mut(&w.tier.tier).unwrap().push(w.name);
                map
            });

        let embed = CreateEmbed::new()
            .title(format!("Tier List for {archetype}"))
            .footer(CreateEmbedFooter::new("From 'Destiny 2: Endgame Analysis'"))
            .fields(TIERS.iter().filter_map(|t| {
                let weapons = weapons.get(t)?;

                if weapons.is_empty() {
                    return None;
                }

                let weapons = weapons
                    .iter()
                    .enumerate()
                    .map(|(i, w)| format!("{}. {}", i + 1, w))
                    .collect::<Vec<_>>();

                Some((t.to_string(), weapons.join("\n"), false))
            }));

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        let tier_option = TIERS.iter().fold(
            CreateCommandOption::new(
                CommandOptionType::String,
                "tier",
                "The tier to display up to",
            ),
            |option, tier| option.add_string_choice(tier.to_string(), tier.to_string()),
        );

        CreateCommand::new("tierlist")
            .description("Get a tier list of weapons from Destiny 2")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "archetype",
                    "The archetype of weapon to display",
                )
                .required(true)
                .set_autocomplete(true),
            )
            .add_option(tier_option)
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "count",
                "The number of weapons to display",
            ))
    }

    pub async fn autocomplete<Data: BungieClientData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
    ) -> Result<()> {
        let weapons: Vec<Weapon> = match std::fs::read_to_string("weapons.json") {
            Ok(weapons) => serde_json::from_str(&weapons).unwrap(),
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
                let weapons = std::fs::read_to_string("weapons.json").unwrap();
                serde_json::from_str(&weapons).unwrap()
            }
        };

        let choices = match option.name {
            "archetype" => weapons
                .iter()
                .map(|w| w.archetype())
                .collect::<HashSet<_>>()
                .into_iter()
                .filter(|t| t.to_lowercase().contains(&option.value.to_lowercase()))
                .map(AutocompleteChoice::from)
                .collect(),
            // "tier" => {
            //     tiers = TIERS
            //         .iter()
            //         .map(|t| AutocompleteChoice::from(t.to_string()))
            //         .collect::<Vec<_>>();
            // }
            _ => Vec::new(),
        };

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(choices),
                ),
            )
            .await
            .unwrap();

        Ok(())
    }
}
