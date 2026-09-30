use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutSummary {
    pub id: i32,
    pub name: String,
    pub class: String,
    pub element: String,
    pub mode: String,
    pub author: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspectForm {
    pub aspect: String,
    pub fragments: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponForm {
    pub name: String,
    pub affinity: String,
    pub archetype: String,
    pub icon_url: String,
    pub perks: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmourForm {
    pub slot: String,
    pub name: String,
    pub icon_url: String,
    pub mods: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatForm {
    pub stat: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutForm {
    pub id: Option<i32>,
    pub name: String,
    pub class: String,
    pub element: String,
    pub mode: String,
    pub tags: Vec<String>,
    pub super_name: String,
    pub super_emoji: String,
    pub class_ability: String,
    pub jump: String,
    pub melee: String,
    pub grenade: String,
    pub aspects: Vec<AspectForm>,
    pub weapons: Vec<WeaponForm>,
    pub armour: Vec<ArmourForm>,
    pub stats: Vec<StatForm>,
    pub artifact_name: String,
    pub artifact_perks: Vec<String>,
    pub author: String,
    pub dim_link: String,
    pub video_url: String,
    pub how_it_works: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogWeaponInfo {
    pub name: String,
    pub affinity: String,
    pub archetype: String,
    pub icon_url: String,
    pub known_perks: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutOptions {
    pub classes: Vec<String>,
    pub elements: Vec<String>,
    pub modes: Vec<String>,
    pub affinities: Vec<String>,
    pub archetypes: Vec<String>,
    pub stats: Vec<String>,
    pub armour_slots: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutCatalog {
    pub weapons: Vec<CatalogWeaponInfo>,
    pub perks: Vec<String>,
    pub emojis: Vec<EmojiInfo>,
    pub usage: Vec<UsageInfo>,
    pub super_names: Vec<(String, String)>,
    pub armour: Vec<ArmourPieceInfo>,
    pub options: LoadoutOptions,
    pub blank: LoadoutForm,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmojiInfo {
    pub name: String,
    pub id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageInfo {
    pub field: String,
    pub key: String,
    pub class: String,
    pub element: String,
    pub uses: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmourPieceInfo {
    pub slot: String,
    pub class: String,
    pub name: String,
    pub icon_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutCheck {
    pub components: u32,
    pub max_components: u32,
    pub text: Option<u32>,
    pub max_text: u32,
    pub error: Option<String>,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmojiSource {
    Url(String),
    DataUri(String),
}
