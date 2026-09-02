pub mod compress;
pub mod decompress;
pub mod dps;
pub mod edit;
pub mod edit_player;
pub mod extract;
pub mod guild;
pub mod gvas;
pub mod palmap;
pub mod player;

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use jiff::Timestamp;

use crate::error::{PalworldError, Result};
use crate::model::{OwnedPal, PlayerName, PlayerRoster, WorldRoster};
use crate::save::dps::StoredPals;

pub const GLOBAL_STORAGE_UID: &str = "00000000000000000000000001000000";

#[must_use]
pub fn mtime_nanos(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| Timestamp::try_from(t).ok())
        .map_or(0, |ts| u64::try_from(ts.as_nanosecond()).unwrap_or(u64::MAX))
}

pub fn validate_level(raw: &[u8]) -> Result<()> {
    let decompressed = decompress::decompress(raw)?;
    let file = gvas::read_gvas(&decompressed)?;
    if !file.properties.0.contains_key("worldSaveData") {
        return Err(PalworldError::Gvas(
            "not a world save: missing worldSaveData".into(),
        ));
    }
    Ok(())
}

pub fn write_level_atomic(dir: &Path, bytes: &[u8]) -> Result<()> {
    write_atomic(&dir.join("Level.sav"), bytes)
}

pub fn write_player_atomic(dir: &Path, uid: &str, bytes: &[u8]) -> Result<()> {
    let stem = uid_to_filename(uid).unwrap_or_else(|| uid.to_string());
    write_raw_player(dir, &stem, bytes)
}

pub fn write_raw_player(dir: &Path, stem: &str, bytes: &[u8]) -> Result<()> {
    write_atomic(&dir.join("Players").join(format!("{stem}.sav")), bytes)
}

fn write_atomic(final_path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = final_path.with_extension("sav.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, final_path)?;
    Ok(())
}

#[must_use]
pub fn player_save_path(save_dir: &Path, uid: &str) -> Option<PathBuf> {
    Some(save_dir.join("Players").join(format!("{}.sav", uid_to_filename(uid)?)))
}

fn read_level(
    save_dir: &Path,
) -> Result<(extract::ExtractedWorld, guild::GuildData)> {
    let raw = std::fs::read(save_dir.join("Level.sav"))?;
    let decompressed = decompress::decompress(&raw)?;
    let level = gvas::read_gvas(&decompressed)?;
    let extracted = extract::extract(&level)?;
    let guilds = guild::decode_guilds(&level);
    Ok((extracted, guilds))
}

fn roster_uids<'a>(
    players: &HashMap<String, extract::PlayerInfo>,
    guilds: &guild::GuildData,
    pal_owners: impl Iterator<Item = &'a String>,
    save_dir: &Path,
) -> Vec<String> {
    let mut uids: Vec<String> = players.keys().cloned().collect();
    uids.extend(pal_owners.cloned());
    uids.extend(guilds.all_members().cloned());
    uids.extend(player_dir_uids(save_dir));
    uids.sort_unstable();
    uids.dedup();
    uids.retain(|uid| uid != GLOBAL_STORAGE_UID);
    uids
}

pub fn load_player_names(save_dir: &Path) -> Result<Vec<PlayerName>> {
    let (extracted, guilds) = read_level(save_dir)?;
    let uids =
        roster_uids(&extracted.players, &guilds, extracted.pals.keys(), save_dir);

    let mut players: Vec<PlayerName> = uids
        .into_iter()
        .map(|uid| {
            let name = extracted
                .players
                .get(&uid)
                .map_or_else(|| uid.clone(), |i| i.name.clone());
            PlayerName::new(uid, name)
        })
        .collect();

    players.sort_by(|a, b| a.search_key.cmp(&b.search_key));
    Ok(players)
}

pub fn load_world(save_dir: &Path) -> Result<WorldRoster> {
    let (stored, level) =
        rayon::join(|| dps::load_all(save_dir), || read_level(save_dir));
    Ok(world_roster(save_dir, level?, stored))
}

pub fn load_world_with<S: BuildHasher>(
    save_dir: &Path,
    stored: HashMap<String, Vec<OwnedPal>, S>,
) -> Result<WorldRoster> {
    Ok(world_roster(save_dir, read_level(save_dir)?, stored))
}

fn world_roster<S: BuildHasher>(
    save_dir: &Path,
    (extracted, guilds): (extract::ExtractedWorld, guild::GuildData),
    stored: HashMap<String, Vec<OwnedPal>, S>,
) -> WorldRoster {
    let extract::ExtractedWorld { players: info, mut pals, base_pals } = extracted;

    pals.remove(GLOBAL_STORAGE_UID);
    for (uid, mut owned) in stored {
        pals.entry(uid).or_default().append(&mut owned);
    }

    let personal: StoredPals = pals.clone();
    let mut pals_by_uid: StoredPals = pals;

    for base in &base_pals {
        match guilds.guild_of(&base.last_owner) {
            Some(gid) => {
                for member in guilds.members(gid) {
                    pals_by_uid
                        .entry(member.clone())
                        .or_default()
                        .push(base.pal.clone());
                }
            },
            None => {
                pals_by_uid
                    .entry(base.last_owner.clone())
                    .or_default()
                    .push(base.pal.clone());
            },
        }
    }

    let uids = roster_uids(&info, &guilds, pals_by_uid.keys(), save_dir);

    let mut players: Vec<PlayerRoster> = uids
        .into_iter()
        .map(|uid| {
            let info = info.get(&uid);
            PlayerRoster {
                name: info.map_or_else(|| uid.clone(), |i| i.name.clone()),
                level: info.map_or(0, |i| i.level),
                exp: info.map_or(0, |i| i.exp),
                personal_pals: personal.get(&uid).cloned().unwrap_or_default(),
                pals: pals_by_uid.get(&uid).cloned().unwrap_or_default(),
                uid,
            }
        })
        .collect();

    players.sort_by_key(|p| p.name.to_lowercase());

    WorldRoster { players }
}

fn player_dir_uids(save_dir: &Path) -> Vec<String> {
    let players_dir = save_dir.join("Players");
    let Ok(entries) = std::fs::read_dir(&players_dir) else {
        return Vec::new();
    };

    let mut uids = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(stem) = name.strip_suffix(".sav") else { continue };
        if stem.ends_with("_dps") {
            continue;
        }
        if let Some(uid) = uid_to_filename(stem) {
            uids.push(uid);
        }
    }
    uids
}

#[must_use]
pub fn uid_to_filename(uid: &str) -> Option<String> {
    let parsed: Option<Vec<u8>> = (0..16)
        .map(|i| {
            uid.get(i * 2..i * 2 + 2).and_then(|p| u8::from_str_radix(p, 16).ok())
        })
        .collect();
    let mut bytes: [u8; 16] = parsed?.try_into().ok()?;

    if let Some(g) = bytes.get_mut(0..4) {
        g.reverse();
    }
    if let Some(g) = bytes.get_mut(4..6) {
        g.reverse();
    }
    if let Some(g) = bytes.get_mut(6..8) {
        g.reverse();
    }
    Some(extract::hex_upper(&bytes))
}
