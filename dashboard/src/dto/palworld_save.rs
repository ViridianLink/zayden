use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct SaveRoster {
    pub level_modified: i64,
    pub trait_ids: Vec<String>,
    pub players: Vec<SavePlayer>,
    pub base_pals: Vec<SavePal>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavePlayer {
    pub instance_id: String,
    pub player_uid: String,
    pub name: String,
    pub level: i32,
    pub pals: Vec<SavePal>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavePal {
    pub instance_id: String,
    pub species: String,
    pub nickname: Option<String>,
    pub gender: String,
    pub stars: u8,
    pub is_lucky: bool,
    pub is_alpha: bool,
    pub level: i32,
    pub talent_hp: u8,
    pub talent_shot: u8,
    pub talent_defense: u8,
    pub traits: Vec<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SaveEdits {
    pub player_edits: Vec<PlayerEdit>,
    pub pal_edits: Vec<PalEdit>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerEdit {
    pub instance_id: String,
    pub level: Option<i32>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PalEdit {
    pub instance_id: String,
    pub level: Option<i32>,
    pub talent_hp: Option<u8>,
    pub talent_shot: Option<u8>,
    pub talent_defense: Option<u8>,
    pub traits: Option<Vec<String>>,
}

#[cfg(feature = "ssr")]
impl From<palworld::save::edit::SavePal> for SavePal {
    fn from(p: palworld::save::edit::SavePal) -> Self {
        Self {
            instance_id: p.instance_id,
            species: p.species,
            nickname: p.nickname,
            gender: p.gender,
            stars: p.stars,
            is_lucky: p.is_lucky,
            is_alpha: p.is_alpha,
            level: p.level,
            talent_hp: p.talent_hp,
            talent_shot: p.talent_shot,
            talent_defense: p.talent_defense,
            traits: p.traits,
        }
    }
}

#[cfg(feature = "ssr")]
impl From<palworld::save::edit::SavePlayer> for SavePlayer {
    fn from(p: palworld::save::edit::SavePlayer) -> Self {
        Self {
            instance_id: p.instance_id,
            player_uid: p.player_uid,
            name: p.name,
            level: p.level,
            pals: p.pals.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(feature = "ssr")]
impl From<palworld::save::edit::SaveRoster> for SaveRoster {
    fn from(r: palworld::save::edit::SaveRoster) -> Self {
        Self {
            level_modified: r.level_modified,
            trait_ids: r.trait_ids,
            players: r.players.into_iter().map(Into::into).collect(),
            base_pals: r.base_pals.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(feature = "ssr")]
impl From<PlayerEdit> for palworld::save::edit::PlayerEdit {
    fn from(e: PlayerEdit) -> Self {
        Self { instance_id: e.instance_id, level: e.level }
    }
}

#[cfg(feature = "ssr")]
impl From<PalEdit> for palworld::save::edit::PalEdit {
    fn from(e: PalEdit) -> Self {
        Self {
            instance_id: e.instance_id,
            level: e.level,
            talent_hp: e.talent_hp,
            talent_shot: e.talent_shot,
            talent_defense: e.talent_defense,
            traits: e.traits,
        }
    }
}

#[cfg(feature = "ssr")]
impl From<SaveEdits> for palworld::save::edit::SaveEdits {
    fn from(e: SaveEdits) -> Self {
        Self {
            player_edits: e.player_edits.into_iter().map(Into::into).collect(),
            pal_edits: e.pal_edits.into_iter().map(Into::into).collect(),
        }
    }
}
