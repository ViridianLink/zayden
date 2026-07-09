use jiff::civil::Weekday;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stat {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Weapon {
    pub slug: String,
    pub name: String,
    pub weapon_type: Option<String>,
    pub ammo_type: Option<String>,
    pub damage: Option<String>,
    pub fire_rate: Option<String>,
    pub magazine_size: Option<String>,
    pub reload_speed: Option<String>,
    pub range: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub stats: Vec<Stat>,
    pub attachment_slots: Vec<AttachmentSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentSlot {
    pub slot: String,
    pub attachment: Option<Attachment>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attachment {
    pub slug: String,
    pub name: String,
    pub slot: Option<String>,
    pub effect: Option<String>,
    pub compatible_weapons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ability {
    pub ability_type: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub cooldown_seconds: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Runner {
    pub slug: String,
    pub name: String,
    pub role: Option<String>,
    pub description: Option<String>,
    pub portrait_url: Option<String>,
    pub abilities: Vec<Ability>,
    pub cores: Vec<String>,
    pub stats: Vec<Stat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CradleNode {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cradle {
    pub description: Option<String>,
    pub nodes: Vec<CradleNode>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildRecipe {
    pub slug: String,
    pub name: String,
    pub shell: Option<String>,
    pub cradle_focus: Option<String>,
    pub gear: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapStatus {
    Available,
    Locked,
    Duo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Poi {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootRoom {
    pub name: String,
    pub location_hint: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarathonMap {
    pub slug: String,
    pub name: String,
    pub status: Option<MapStatus>,
    pub map_image_url: Option<String>,
    pub pois: Vec<Poi>,
    pub extractions: Vec<Location>,
    pub event_spawns: Vec<Location>,
    pub keycard_rooms: Vec<LootRoom>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contract {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub difficulty: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgrade {
    pub name: String,
    pub cost: Option<String>,
    pub requirements: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Faction {
    pub slug: String,
    pub name: String,
    pub priority_contracts: Vec<Contract>,
    pub upgrades: Vec<Upgrade>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaEntry {
    pub item: String,
    pub tier: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotationWindow {
    pub start_weekday: Weekday,
    pub start_hour_pt: u8,
    pub end_weekday: Weekday,
    pub end_hour_pt: u8,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub ranked_window: RotationWindow,
    pub cryo_window: RotationWindow,
    pub duo_map_pool: Vec<String>,
    pub weekly_game_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsItem {
    pub feed_key: String,
    pub id: String,
    pub source_label: String,
    pub title: String,
    pub url: Option<String>,
    pub summary: Option<String>,
}
