use std::collections::HashMap;

use bungie_api::DestinyInventoryItemDefinition;
use destiny2_core::BungieClientData;
use google_sheets_api::SheetsClientBuilder;
use google_sheets_api::types::common::Color;
use google_sheets_api::types::sheet::GridData;

pub mod affinity;
pub mod frame;
pub mod tier;
pub mod weapon;

pub use affinity::Affinity;
pub use frame::Frame;
pub use tier::{TIERS, Tier, TierLabel};
use tracing::error;
pub use weapon::{Weapon, WeaponBuilder};

use crate::Result;

const ENDGAME_ANALYSIS_ID: &str = "1JM-0SlxVDAi-C6rGVlLxa-J1WGewEeL8Qvq4htWZHhY";

fn primary_colour(color: &Color) -> bool {
    (color.red - 0.952_941_2).abs() < f64::EPSILON
        && (color.green - 0.952_941_2).abs() < f64::EPSILON
        && (color.blue - 0.952_941_2).abs() < f64::EPSILON
}

fn special_colour(color: &Color) -> bool {
    color.red == 0.0 && (color.green - 1.0).abs() < f64::EPSILON && color.blue == 0.0
}

fn heavy_colour(color: &Color) -> bool {
    (color.red - 0.6).abs() < f64::EPSILON
        && color.green == 0.0
        && (color.blue - 1.0).abs() < f64::EPSILON
}

pub struct EndgameAnalysisSheet;

impl EndgameAnalysisSheet {
    pub async fn item_manifest<Data: BungieClientData>(
        data: &Data,
    ) -> HashMap<String, DestinyInventoryItemDefinition> {
        let client = data.bungie_client();
        let manifest = match client.destiny_manifest().await {
            Ok(manifest) => manifest,
            Err(e) => {
                error!("Destiny Manifest Error: {e}");
                return HashMap::new();
            },
        };

        match client.destiny_inventory_item_definition(&manifest, "en").await {
            Ok(manifest) => manifest,
            Err(e) => {
                error!("Destiny item definition error: {e}");
                HashMap::new()
            },
        }
    }

    pub async fn update(
        manifest: &HashMap<String, DestinyInventoryItemDefinition>,
        api_key: &str,
    ) -> Result<()> {
        let client =
            SheetsClientBuilder::new(api_key).build().expect("data invariant");

        let spreadsheet = client.spreadsheet(ENDGAME_ANALYSIS_ID, true).await?;

        let weapons = spreadsheet
            .sheets
            .into_iter()
            .filter(|s| !s.properties.hidden)
            .filter(|s| {
                primary_colour(&s.properties.tab_color)
                    || special_colour(&s.properties.tab_color)
                    || heavy_colour(&s.properties.tab_color)
                    || s.properties.title == "Other"
            })
            .map(|mut sheet| {
                (sheet.properties.title, sheet.data.pop().expect("data invariant"))
            })
            .flat_map(|(title, data)| {
                Self::parse_weapon_data(manifest, &title, data)
            })
            .collect::<Vec<_>>();

        let json = serde_json::to_string(&weapons).expect("data invariant");
        std::fs::write("weapons.json", json).expect("data invariant");

        Ok(())
    }

    fn parse_weapon_data(
        manifest: &HashMap<String, DestinyInventoryItemDefinition>,
        title: &str,
        data: GridData,
    ) -> Vec<Weapon> {
        let mut iter = data.row_data.into_iter().skip(1);
        let header = iter.next().expect("data invariant");
        iter.filter_map(|r| WeaponBuilder::from_row(title, &header, r))
            .map(|builder| {
                let item = match manifest.values().find(|item| {
                    item.display_properties.name.eq_ignore_ascii_case(&builder.name)
                }) {
                    Some(item) => item,
                    None => {
                        error!("Missing item: {}", builder.name);
                        &DestinyInventoryItemDefinition::default()
                    },
                };

                builder.build(item)
            })
            .collect()
    }
}
