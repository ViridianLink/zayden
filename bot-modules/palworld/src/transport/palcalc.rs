use reqwest::Client;
use serde::Deserialize;

use super::BreedingMap;
use crate::error::Result;

pub const DEFAULT_PALCALC_BASE: &str =
    "https://raw.githubusercontent.com/tylercamp/palcalc/main/PalCalc.Model";

const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                                   (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RawPalCalcId {
    #[serde(rename = "PalDexNo")]
    pub pal_dex_no: i64,
    #[serde(rename = "IsVariant", default)]
    pub is_variant: bool,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawWorkSuitability {
    #[serde(default)]
    pub kindling: i64,
    #[serde(default)]
    pub watering: i64,
    #[serde(default)]
    pub planting: i64,
    #[serde(default)]
    pub generate_electricity: i64,
    #[serde(default)]
    pub handiwork: i64,
    #[serde(default)]
    pub gathering: i64,
    #[serde(default)]
    pub lumbering: i64,
    #[serde(default)]
    pub mining: i64,
    #[serde(default)]
    pub medicine_production: i64,
    #[serde(default)]
    pub cooling: i64,
    #[serde(default)]
    pub transporting: i64,
    #[serde(default)]
    pub farming: i64,
}

impl RawWorkSuitability {
    #[must_use]
    pub fn entries(&self) -> Vec<(&'static str, i64)> {
        [
            ("Kindling", self.kindling),
            ("Watering", self.watering),
            ("Planting", self.planting),
            ("Generate Electricity", self.generate_electricity),
            ("Handiwork", self.handiwork),
            ("Gathering", self.gathering),
            ("Lumbering", self.lumbering),
            ("Mining", self.mining),
            ("Medicine Production", self.medicine_production),
            ("Cooling", self.cooling),
            ("Transporting", self.transporting),
            ("Farming", self.farming),
        ]
        .into_iter()
        .filter(|&(_, level)| level > 0)
        .collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawPalCalcPal {
    pub id: RawPalCalcId,
    pub name: String,
    pub internal_name: String,
    #[serde(default)]
    pub breeding_power: i64,
    #[serde(default)]
    pub breeding_power_priority: i64,
    pub price: Option<i64>,
    pub min_wild_level: Option<i64>,
    pub max_wild_level: Option<i64>,
    pub rarity: Option<i64>,
    pub size: Option<String>,
    #[serde(default)]
    pub nocturnal: bool,
    #[serde(default)]
    pub hp: i64,
    #[serde(default)]
    pub defense: i64,
    #[serde(default)]
    pub attack: i64,
    #[serde(default)]
    pub walk_speed: i64,
    #[serde(default)]
    pub run_speed: i64,
    #[serde(default)]
    pub ride_sprint_speed: i64,
    #[serde(default)]
    pub transport_speed: i64,
    #[serde(default)]
    pub stamina: i64,
    #[serde(default)]
    pub food_amount: i64,
    #[serde(default)]
    pub work_suitability: RawWorkSuitability,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawPalCalcDb {
    pals: Vec<RawPalCalcPal>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawBreedRecord {
    #[serde(rename = "Parent1InternalName")]
    parent1: String,
    #[serde(rename = "Parent2InternalName")]
    parent2: String,
    #[serde(rename = "ChildInternalName")]
    child: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawBreedingTable {
    breeding: Vec<RawBreedRecord>,
}

fn breeding_map(table: impl IntoIterator<Item = [String; 3]>) -> BreedingMap {
    let mut map: BreedingMap = BreedingMap::new();
    for [parent1, parent2, child] in table {
        let pairs = map.entry(child).or_default();
        let entry = [parent1, parent2];
        if !pairs.contains(&entry) {
            pairs.push(entry);
        }
    }
    map
}

pub fn parse_pals(json: &str) -> serde_json::Result<Vec<RawPalCalcPal>> {
    Ok(serde_json::from_str::<RawPalCalcDb>(json)?.pals)
}

pub fn parse_breeding(json: &str) -> serde_json::Result<BreedingMap> {
    let table: RawBreedingTable = serde_json::from_str(json)?;
    Ok(breeding_map(
        table.breeding.into_iter().map(|r| [r.parent1, r.parent2, r.child]),
    ))
}

pub struct PalCalc {
    client: Client,
    base: String,
}

impl PalCalc {
    #[must_use]
    pub fn new(client: Client, base: Option<String>) -> Self {
        Self {
            client,
            base: base.unwrap_or_else(|| DEFAULT_PALCALC_BASE.to_string()),
        }
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, file: &str) -> Result<T> {
        let url = format!("{}/{file}", self.base.trim_end_matches('/'));
        let value = self
            .client
            .get(&url)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?;
        Ok(value)
    }

    pub async fn pals(&self) -> Result<Vec<RawPalCalcPal>> {
        Ok(self.get::<RawPalCalcDb>("db.json").await?.pals)
    }

    pub async fn breeding(&self) -> Result<BreedingMap> {
        let table = self.get::<RawBreedingTable>("breeding.json").await?;
        Ok(breeding_map(
            table.breeding.into_iter().map(|r| [r.parent1, r.parent2, r.child]),
        ))
    }
}
