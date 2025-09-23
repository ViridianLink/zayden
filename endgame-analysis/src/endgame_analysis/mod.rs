use std::collections::HashMap;
use std::env;

use bungie_api::DestinyInventoryItemDefinition;
use google_sheets_api::SheetsClientBuilder;
use google_sheets_api::types::common::Color;
use google_sheets_api::types::sheet::GridData;

pub mod affinity;
pub mod frame;
pub mod tier;
pub mod weapon;

pub use affinity::Affinity;
pub use frame::Frame;
pub use tier::Tier;
pub use tier::{TIERS, TierLabel};
pub use weapon::{Weapon, WeaponBuilder};

use crate::Result;

const ENDGAME_ANALYSIS_ID: &str = "1JM-0SlxVDAi-C6rGVlLxa-J1WGewEeL8Qvq4htWZHhY";

fn primary_colour(color: &Color) -> bool {
    color.red == 0.9529412 && color.green == 0.9529412 && color.blue == 0.9529412
}

fn special_colour(color: &Color) -> bool {
    color.red == 0.0 && color.green == 1.0 && color.blue == 0.0
}

fn heavy_colour(color: &Color) -> bool {
    color.red == 0.6 && color.green == 0.0 && color.blue == 1.0
}

pub struct EndgameAnalysisSheet;

impl EndgameAnalysisSheet {
    pub async fn update(manifest: &HashMap<String, DestinyInventoryItemDefinition>) -> Result<()> {
        let api_key = env::var("GOOGLE_API_KEY").unwrap();

        let client = SheetsClientBuilder::new(api_key).build().unwrap();

        let spreadsheet = client.spreadsheet(ENDGAME_ANALYSIS_ID, true).await.unwrap();

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
            .map(|mut sheet| (sheet.properties.title, sheet.data.pop().unwrap()))
            .flat_map(|(title, data)| Self::parse_weapon_data(manifest, title, data))
            .collect::<Vec<_>>();

        let json = serde_json::to_string(&weapons).unwrap();
        std::fs::write("weapons.json", json).unwrap();

        Ok(())
    }

    fn parse_weapon_data(
        manifest: &HashMap<String, DestinyInventoryItemDefinition>,
        title: String,
        data: GridData,
    ) -> Vec<Weapon> {
        let mut iter = data.row_data.into_iter().skip(1);
        let header = iter.next().unwrap();
        iter.filter_map(|r| WeaponBuilder::from_row(&title, &header, r))
            .map(|builder| {
                let item = manifest
                    .values()
                    .find(|item| {
                        item.display_properties
                            .name
                            .eq_ignore_ascii_case(&builder.name)
                    })
                    .unwrap_or_else(|| panic!("Missing item: {}", builder.name));

                builder.build(item)
            })
            .collect()
    }
}
