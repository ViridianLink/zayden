use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;

use crate::error::Result;

pub const DEFAULT_BASE: &str =
    "https://raw.githubusercontent.com/mlg404/palworld-paldex-api/main/src";

const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                                   (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

pub type BreedingMap = HashMap<String, Vec<[String; 2]>>;

#[derive(Debug, Clone, Deserialize)]
pub struct RawType {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawSuitability {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub level: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawAura {
    #[serde(default)]
    pub name: String,
    pub description: Option<String>,
    pub tech: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawSkill {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub level: i64,
    #[serde(rename = "type")]
    pub element: Option<String>,
    pub cooldown: Option<i64>,
    pub power: Option<i64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct RawAttack {
    #[serde(default)]
    pub melee: i64,
    #[serde(default)]
    pub ranged: i64,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct RawSpeed {
    #[serde(default)]
    pub ride: i64,
    #[serde(default)]
    pub run: i64,
    #[serde(default)]
    pub walk: i64,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct RawStats {
    #[serde(default)]
    pub hp: i64,
    #[serde(default)]
    pub attack: RawAttack,
    #[serde(default)]
    pub defense: i64,
    #[serde(default)]
    pub speed: RawSpeed,
    #[serde(default)]
    pub stamina: i64,
    #[serde(default)]
    pub support: i64,
    #[serde(default)]
    pub food: i64,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct RawBreeding {
    pub rank: Option<i64>,
    pub order: Option<i64>,
    #[serde(rename = "child_eligble", default)]
    pub child_eligible: bool,
    pub male_probability: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPal {
    #[serde(default)]
    pub id: i64,
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub types: Vec<RawType>,
    #[serde(default)]
    pub suitability: Vec<RawSuitability>,
    #[serde(default)]
    pub drops: Vec<String>,
    pub aura: Option<RawAura>,
    pub description: Option<String>,
    #[serde(default)]
    pub skills: Vec<RawSkill>,
    pub stats: Option<RawStats>,
    pub genus: Option<String>,
    pub rarity: Option<i64>,
    pub price: Option<i64>,
    pub size: Option<String>,
    pub breeding: Option<RawBreeding>,
    #[serde(rename = "imageWiki")]
    pub image_wiki: Option<String>,
    pub wiki: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawItem {
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub description: Option<String>,
    pub gold: Option<i64>,
    pub weight: Option<i64>,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPassive {
    pub name: String,
    pub positive: Option<String>,
    pub negative: Option<String>,
    #[serde(default)]
    pub tier: i64,
}

pub struct Paldex {
    client: Client,
    base: String,
}

impl Paldex {
    #[must_use]
    pub fn new(client: Client, base: Option<String>) -> Self {
        Self { client, base: base.unwrap_or_else(|| DEFAULT_BASE.to_string()) }
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

    pub async fn pals(&self) -> Result<Vec<RawPal>> {
        self.get("pals.json").await
    }

    pub async fn breeding(&self) -> Result<BreedingMap> {
        self.get("breeding.json").await
    }

    pub async fn items(&self) -> Result<Vec<RawItem>> {
        self.get("item.json").await
    }

    pub async fn passives(&self) -> Result<HashMap<String, RawPassive>> {
        self.get("passive_skills.json").await
    }
}
