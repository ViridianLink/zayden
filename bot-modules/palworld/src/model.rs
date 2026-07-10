#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Element {
    Neutral,
    Fire,
    Water,
    Grass,
    Electric,
    Ice,
    Ground,
    Dark,
    Dragon,
}

impl Element {
    #[must_use]
    pub const fn all() -> [Self; 9] {
        [
            Self::Neutral,
            Self::Fire,
            Self::Water,
            Self::Grass,
            Self::Electric,
            Self::Ice,
            Self::Ground,
            Self::Dark,
            Self::Dragon,
        ]
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Neutral => "Neutral",
            Self::Fire => "Fire",
            Self::Water => "Water",
            Self::Grass => "Grass",
            Self::Electric => "Electric",
            Self::Ice => "Ice",
            Self::Ground => "Ground",
            Self::Dark => "Dark",
            Self::Dragon => "Dragon",
        }
    }

    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Fire => "fire",
            Self::Water => "water",
            Self::Grass => "grass",
            Self::Electric => "electric",
            Self::Ice => "ice",
            Self::Ground => "ground",
            Self::Dark => "dark",
            Self::Dragon => "dragon",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_lowercase().as_str() {
            "neutral" | "normal" | "none" => Some(Self::Neutral),
            "fire" | "flame" => Some(Self::Fire),
            "water" | "aqua" => Some(Self::Water),
            "grass" | "leaf" | "plant" => Some(Self::Grass),
            "electric" | "electricity" | "lightning" | "electricty" => {
                Some(Self::Electric)
            },
            "ice" | "frost" => Some(Self::Ice),
            "ground" | "earth" => Some(Self::Ground),
            "dark" | "shadow" => Some(Self::Dark),
            "dragon" => Some(Self::Dragon),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub hp: i64,
    pub attack_melee: i64,
    pub attack_ranged: i64,
    pub defense: i64,
    pub speed_ride: i64,
    pub speed_run: i64,
    pub speed_walk: i64,
    pub stamina: i64,
    pub support: i64,
    pub food: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Suitability {
    pub kind: String,
    pub level: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Aura {
    pub name: String,
    pub description: Option<String>,
    pub tech: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSkill {
    pub name: String,
    pub level: i64,
    pub element: Option<String>,
    pub cooldown: Option<i64>,
    pub power: Option<i64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pal {
    pub key: String,
    pub paldex_no: i64,
    pub name: String,
    pub elements: Vec<Element>,
    pub stats: Option<Stats>,
    pub suitability: Vec<Suitability>,
    pub drops: Vec<String>,
    pub partner_skill: Option<Aura>,
    pub active_skills: Vec<ActiveSkill>,
    pub description: Option<String>,
    pub genus: Option<String>,
    pub rarity: Option<i64>,
    pub price: Option<i64>,
    pub size: Option<String>,
    pub breeding_rank: Option<i64>,
    pub breeding_order: Option<i64>,
    pub child_eligible: bool,
    pub male_probability: Option<f64>,
    pub image_url: Option<String>,
    pub wiki_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Item {
    pub key: String,
    pub name: String,
    pub item_type: Option<String>,
    pub description: Option<String>,
    pub gold: Option<i64>,
    pub weight: Option<i64>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PassiveSkill {
    pub key: String,
    pub name: String,
    pub positive: Option<String>,
    pub negative: Option<String>,
    pub tier: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BreedingChild {
    pub child: Pal,
    pub unique: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentPair {
    pub a: String,
    pub b: String,
}
