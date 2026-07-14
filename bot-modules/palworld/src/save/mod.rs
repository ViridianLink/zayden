//! World-save parsing: decompress `Level.sav`, walk its Pal map, and merge
//! with the `Players/` directory into a [`WorldRoster`].
//!
//! [`load_world`] is the only public entry point. It performs blocking file IO
//! and CPU-heavy decompression/parsing, so callers must run it off the async
//! runtime (see `client::PalworldClient::roster`).

pub mod decompress;
pub mod extract;
pub mod gvas;
pub mod palmap;

use std::path::Path;

use crate::error::Result;
use crate::model::{PlayerRoster, WorldRoster};

pub fn load_world(save_dir: &Path) -> Result<WorldRoster> {
    let level_path = save_dir.join("Level.sav");
    let raw = std::fs::read(&level_path)?;
    let decompressed = decompress::decompress(&raw)?;
    let level = gvas::read_gvas(&decompressed)?;
    let extracted = extract::extract(&level)?;

    let mut uids: Vec<String> = extracted
        .player_names
        .keys()
        .chain(extracted.pals.keys())
        .cloned()
        .collect();
    uids.extend(player_dir_uids(save_dir));
    uids.sort_unstable();
    uids.dedup();

    let mut players: Vec<PlayerRoster> = uids
        .into_iter()
        .map(|uid| {
            let name = extracted
                .player_names
                .get(&uid)
                .cloned()
                .unwrap_or_else(|| uid.clone());
            let pals = extracted.pals.get(&uid).cloned().unwrap_or_default();
            PlayerRoster { uid, name, pals }
        })
        .collect();

    players.sort_by_key(|p| p.name.to_lowercase());

    Ok(WorldRoster { players })
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
        if let Some(uid) = normalize_player_uid(stem) {
            uids.push(uid);
        }
    }
    uids
}

fn normalize_player_uid(stem: &str) -> Option<String> {
    let parsed: Option<Vec<u8>> = (0..16)
        .map(|i| {
            stem.get(i * 2..i * 2 + 2).and_then(|p| u8::from_str_radix(p, 16).ok())
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
