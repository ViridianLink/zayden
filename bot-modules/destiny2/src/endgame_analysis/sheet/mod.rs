use std::collections::HashMap;

use bungie_api::{BungieClient, DestinyInventoryItemDefinition};
use google_sheets_api::SheetsClientBuilder;
use google_sheets_api::types::common::Color;
use google_sheets_api::types::sheet::GridData;
use sqlx::PgPool;

pub mod affinity;
pub mod frame;
pub mod tier;
pub mod weapon;

pub use affinity::Affinity;
pub use frame::Frame;
pub use tier::{TIERS, Tier, TierLabel};
use tracing::error;
pub use weapon::{Weapon, WeaponBuilder};

use crate::db::endgame;
use crate::endgame_analysis::{EndgameAnalysisError, Result};

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
    pub async fn item_manifest(
        client: &BungieClient,
    ) -> HashMap<String, DestinyInventoryItemDefinition> {
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

    pub async fn weapons(
        pool: &PgPool,
        client: &BungieClient,
        api_key: &str,
    ) -> Result<Vec<Weapon>> {
        if endgame::is_empty(pool).await? {
            let manifest = client.destiny_manifest().await?;
            let item_manifest =
                client.destiny_inventory_item_definition(&manifest, "en").await?;
            Self::update(pool, &item_manifest, api_key).await?;
        }

        Ok(endgame::all(pool).await?)
    }

    pub async fn update(
        pool: &PgPool,
        manifest: &HashMap<String, DestinyInventoryItemDefinition>,
        api_key: &str,
    ) -> Result<()> {
        let client = SheetsClientBuilder::new(api_key).build()?;

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
            .filter_map(|mut sheet| {
                let data = sheet.data.pop()?;
                Some((sheet.properties.title, data))
            })
            .filter_map(|(title, data)| {
                Self::parse_weapon_data(manifest, &title, data)
                    .inspect_err(|e| error!("Skipping sheet '{title}': {e}"))
                    .ok()
            })
            .flatten()
            .collect::<Vec<Weapon>>();

        let existing = usize::try_from(endgame::count(pool).await?).unwrap_or(0);
        if !endgame::is_safe_replace(existing, weapons.len()) {
            error!(
                "Refusing endgame refresh: parsed {} weapons vs {existing} \
                 existing — keeping current catalog (likely upstream sheet or \
                 parser drift)",
                weapons.len()
            );
            return Ok(());
        }

        endgame::replace(pool, &weapons).await?;

        Ok(())
    }

    fn parse_weapon_data(
        manifest: &HashMap<String, DestinyInventoryItemDefinition>,
        title: &str,
        data: GridData,
    ) -> Result<Vec<Weapon>> {
        let mut iter = data.row_data.into_iter().skip(1);
        let Some(header) = iter.next() else {
            return Err(EndgameAnalysisError::MissingHeaderRow(title.to_string()));
        };

        let weapons = iter
            .filter_map(|r| {
                WeaponBuilder::from_row(title, &header, r)
                    .inspect_err(|e| error!("Skipping weapon in '{title}': {e}"))
                    .ok()
                    .flatten()
            })
            .filter_map(|builder| {
                let item = match manifest.values().find(|item| {
                    item.display_properties.name.eq_ignore_ascii_case(&builder.name)
                }) {
                    Some(item) => item,
                    None => {
                        error!("Missing item: {}", builder.name);
                        &DestinyInventoryItemDefinition::default()
                    },
                };

                builder
                    .build(item)
                    .inspect_err(|e| {
                        error!("Skipping weapon build in '{title}': {e}");
                    })
                    .ok()
            })
            .collect();

        Ok(weapons)
    }
}
