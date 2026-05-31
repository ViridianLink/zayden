use std::collections::{HashMap, HashSet};
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
    CreateEmbed,
    CreateEmbedFooter,
    CreateInteractionResponse,
    EditInteractionResponse,
    ResolvedOption,
    ResolvedValue,
};
use tokio::sync::RwLock;
use zayden_core::parse_options;

use super::endgame_analysis::tier::TIERS;
use crate::Result;
use crate::endgame_analysis::EndgameAnalysisSheet;
use crate::endgame_analysis::weapon::Weapon;

pub struct TierListCommand;

impl TierListCommand {
    #[expect(
        clippy::significant_drop_tightening,
        clippy::unreachable,
        clippy::wildcard_enum_match_arm,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::indexing_slicing,
        reason = "bungie_client() borrows the read guard; required Discord options guaranteed; integer count from Discord can't overflow usize; TIERS slice access is bounded"
    )]
    pub async fn run<Data: BungieClientData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::String(archetype)) = options.remove("archetype")
        else {
            return Ok(());
        };

        let count = match options.get("count") {
            Some(ResolvedValue::Integer(count)) => {
                usize::try_from(*count).expect("temp")
            },
            _ => usize::MAX,
        };

        let tiers = match options.get("tier") {
            Some(ResolvedValue::String(tier)) => {
                let tier = tier.parse().expect("valid tier string");
                let index = TIERS
                    .iter()
                    .copied()
                    .position(|t| t == tier)
                    .expect("tier always in TIERS list");
                TIERS.get(..=index).expect("temp")
            },
            _ => &TIERS,
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

                EndgameAnalysisSheet::update(&item_manifest).await?;
                let w = fs::read_to_string("weapons.json")
                    .expect("weapons.json readable");
                serde_json::from_str(&w).expect("valid JSON")
            },
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
            .take(count)
            .fold(init_map, |mut map, w| {
                map.get_mut(&w.tier.tier)
                    .expect("tier key always in map")
                    .push(w.name);
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
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        let tier_option = TIERS.iter().fold(
            CreateCommandOption::new(
                CommandOptionType::String,
                "tier",
                "The tier to display up to",
            ),
            |option, tier| {
                option.add_string_choice(tier.to_string(), tier.to_string())
            },
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

    #[expect(
        clippy::significant_drop_tightening,
        reason = "bungie_client() borrows the read guard; cache miss reads require the lock to remain alive across multiple await points"
    )]
    pub async fn autocomplete<Data: BungieClientData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
    ) -> Result<()> {
        let weapons: Vec<Weapon> = match fs::read_to_string("weapons.json") {
            Ok(weapons) => {
                serde_json::from_str(&weapons).expect("valid weapons JSON")
            },
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

                EndgameAnalysisSheet::update(&item_manifest).await?;
                let weapons = fs::read_to_string("weapons.json")
                    .expect("weapons.json readable");
                serde_json::from_str(&weapons).expect("valid weapons JSON")
            },
        };

        let choices = match option.name {
            "archetype" => weapons
                .iter()
                .map(Weapon::archetype)
                .collect::<HashSet<_>>()
                .into_iter()
                .filter(|t| t.to_lowercase().contains(&option.value.to_lowercase()))
                .map(AutocompleteChoice::from)
                .collect(),
            _ => Vec::new(),
        };

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(choices),
                ),
            )
            .await?;

        Ok(())
    }
}
