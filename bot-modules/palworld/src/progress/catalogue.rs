use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    #[default]
    Palpagos,
    #[serde(rename = "tree")]
    WorldTree,
}

impl Region {
    pub const ALL: [Self; 2] = [Self::Palpagos, Self::WorldTree];

    #[must_use]
    pub const fn unlock_flag(self) -> &'static str {
        match self {
            Self::Palpagos => "MainMap",
            Self::WorldTree => "Tree",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Palpagos => "Palpagos Islands",
            Self::WorldTree => "World Tree",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FastTravel {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub map: Region,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Boss {
    pub spawner: String,
    pub character_id: String,
    pub name: String,
    pub alpha: bool,
    pub bounty: bool,
    pub level: i64,
    #[serde(default)]
    pub map: Region,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Relic {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub map: Region,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelicType {
    pub key: String,
    pub name: String,
    pub max_rank: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Technology {
    pub id: String,
    pub name: String,
    pub boss: bool,
    pub level: i64,
    pub cost: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mission {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tower {
    pub id: String,
    pub flag: String,
    pub name: String,
    pub location: String,
    #[serde(default)]
    pub map: Region,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PalEntry {
    pub id: String,
    pub name: String,
    pub dex: i64,
    pub tribe: String,
}

#[derive(Debug)]
pub struct Catalogue {
    pub fast_travel: Vec<FastTravel>,
    pub bosses: Vec<Boss>,
    pub relics: Vec<Relic>,
    pub relic_types: Vec<RelicType>,
    pub technologies: Vec<Technology>,
    pub missions: Vec<Mission>,
    pub areas: Vec<String>,
    pub towers: Vec<Tower>,
    pub pals: Vec<PalEntry>,

    pal_index: HashMap<String, usize>,
    relic_type_index: HashMap<String, usize>,
}

fn embedded<T: DeserializeOwned + Default>(name: &str, json: &str) -> T {
    serde_json::from_str(json).unwrap_or_else(|e| {
        tracing::error!(
            dataset = name,
            error = %e,
            "palworld: embedded catalogue data did not parse; /palworld progress \
             will count this category as empty",
        );
        T::default()
    })
}

static CATALOGUE: LazyLock<Catalogue> = LazyLock::new(|| {
    let pals: Vec<PalEntry> =
        embedded("pals.json", include_str!("../../data/pals.json"));

    let relic_types: Vec<RelicType> =
        embedded("relic_types.json", include_str!("../../data/relic_types.json"));

    Catalogue {
        pal_index: index_by(&pals, |p| &p.id),
        relic_type_index: index_by(&relic_types, |t| &t.key),

        fast_travel: embedded(
            "fast_travel_points.json",
            include_str!("../../data/fast_travel_points.json"),
        ),

        bosses: embedded("bosses.json", include_str!("../../data/bosses.json")),

        relics: embedded("relics.json", include_str!("../../data/relics.json")),

        technologies: embedded(
            "technologies.json",
            include_str!("../../data/technologies.json"),
        ),

        missions: embedded(
            "missions.json",
            include_str!("../../data/missions.json"),
        ),

        areas: embedded("areas.json", include_str!("../../data/areas.json")),

        towers: embedded("towers.json", include_str!("../../data/towers.json")),

        pals,
        relic_types,
    }
});

#[must_use]
pub fn catalogue() -> &'static Catalogue {
    &CATALOGUE
}

fn index_by<T>(items: &[T], key: impl Fn(&T) -> &String) -> HashMap<String, usize> {
    items.iter().enumerate().map(|(i, item)| (key(item).to_lowercase(), i)).collect()
}

const SPECIES_PREFIXES: &[&str] =
    &["BOSS_", "GYM_", "PREDATOR_", "RAID_", "SUMMON_"];

impl Catalogue {
    #[must_use]
    pub fn pal(&self, character_id: &str) -> Option<&PalEntry> {
        let trimmed = character_id.trim();
        if let Some(entry) = self.pal_by_exact_id(trimmed) {
            return Some(entry);
        }

        let base = SPECIES_PREFIXES.iter().find_map(|prefix| {
            trimmed
                .get(..prefix.len())
                .filter(|head| head.eq_ignore_ascii_case(prefix))
                .and_then(|_| trimmed.get(prefix.len()..))
        })?;
        self.pal_by_exact_id(base)
    }

    fn pal_by_exact_id(&self, id: &str) -> Option<&PalEntry> {
        self.pal_index.get(&id.to_lowercase()).and_then(|i| self.pals.get(*i))
    }

    #[must_use]
    pub fn relic_type(&self, key: &str) -> Option<&RelicType> {
        self.relic_type_index
            .get(&key.to_lowercase())
            .and_then(|i| self.relic_types.get(*i))
    }

    pub fn effigies(&self) -> impl Iterator<Item = &Relic> {
        self.relics.iter().filter(|r| r.kind == "capture_power")
    }

    pub fn fast_travel_on(&self, map: Region) -> impl Iterator<Item = &FastTravel> {
        self.fast_travel.iter().filter(move |p| p.map == map)
    }

    pub fn bosses_on(&self, map: Region) -> impl Iterator<Item = &Boss> {
        self.bosses.iter().filter(move |b| b.map == map)
    }

    pub fn towers_on(&self, map: Region) -> impl Iterator<Item = &Tower> {
        self.towers.iter().filter(move |t| t.map == map)
    }

    pub fn relics_on(&self, map: Region) -> impl Iterator<Item = &Relic> {
        self.relics.iter().filter(move |r| r.map == map)
    }

    pub fn effigies_on(&self, map: Region) -> impl Iterator<Item = &Relic> {
        self.effigies().filter(move |r| r.map == map)
    }

    pub fn tracked_missions(&self) -> impl Iterator<Item = &Mission> {
        self.missions.iter().filter(|m| m.kind != "hidden")
    }
}
