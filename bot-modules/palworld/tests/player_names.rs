//! The name-only roster path that autocomplete runs on.
//!
//! `/palworld roster`, `breed-plan` and `link` complete player names against a
//! save. They used to do it by building a full [`WorldRoster`], which parses
//! every `Players/*_dps.sav` - each ~34 KB on disk but ~73 MB decompressed, at
//! roughly half a second apiece. That reliably overran Discord's 3-second
//! interaction window.
//!
//! `load_player_names` stops after the `Level.sav` decode, where the names
//! actually live. These tests pin the property that makes the swap safe: it
//! must offer the user exactly the same choices as the full roster did.

use std::fs;

use palworld::save::{load_player_names, load_world};

pub mod common;
use common::{progressed_world, storage_world};

/// The whole point: the cheap path and the expensive one name the same players,
/// in the same order. If this diverges, autocomplete starts offering names that
/// the command backing it can't resolve.
#[test]
fn the_name_index_matches_the_full_roster() {
    for dir in [progressed_world(), storage_world()] {
        let roster = load_world(&dir).expect("load_world");
        let names = load_player_names(&dir).expect("load_player_names");

        let from_roster: Vec<(&str, &str)> = roster
            .players
            .iter()
            .map(|p| (p.uid.as_str(), p.name.as_str()))
            .collect();
        let from_index: Vec<(&str, &str)> =
            names.iter().map(|p| (p.uid.as_str(), p.name.as_str())).collect();

        assert_eq!(
            from_index,
            from_roster,
            "name index diverged from the full roster for {}",
            dir.display(),
        );
    }
}

/// `search_key` is the lowercased name, precomputed so per-keystroke filtering
/// over the player list allocates nothing.
#[test]
fn every_entry_carries_a_prelowercased_search_key() {
    let names = load_player_names(&progressed_world()).expect("load_player_names");
    assert!(!names.is_empty(), "fixture yields players");

    for player in &names {
        assert_eq!(player.search_key, player.name.to_lowercase());
    }
}

/// Entries are ordered by the key autocomplete filters on, so the 25-choice
/// truncation is stable rather than dependent on hash iteration order.
#[test]
fn entries_are_sorted_by_search_key() {
    let names = load_player_names(&storage_world()).expect("load_player_names");
    let mut sorted = names.clone();
    sorted.sort_by(|a, b| a.search_key.cmp(&b.search_key));
    assert_eq!(names, sorted);
}

/// The reason this path exists, asserted structurally rather than by the
/// clock: with every `_dps.sav` removed, the name index is unchanged.
///
/// Those files are the entire cost difference - ~530 ms each, and four of them
/// in this fixture. If the names still come out identical without them, the
/// expensive decode genuinely is off the autocomplete path. A wall-clock
/// assertion would test the same thing far less reliably.
#[test]
fn the_name_index_does_not_read_pal_storage() {
    let source = progressed_world();
    let scratch =
        std::env::temp_dir().join(format!("palworld-names-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(scratch.join("Players")).expect("scratch dir");

    fs::copy(source.join("Level.sav"), scratch.join("Level.sav")).expect("level");

    let mut skipped = 0;
    for entry in fs::read_dir(source.join("Players")).expect("players").flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with("_dps.sav") {
            skipped += 1;
            continue;
        }
        fs::copy(entry.path(), scratch.join("Players").join(name.as_ref()))
            .expect("player save");
    }
    assert!(skipped > 0, "fixture must contain Pal storage saves to omit");

    let with_storage = load_player_names(&source).expect("with storage");
    let without_storage = load_player_names(&scratch).expect("without storage");

    let _ = fs::remove_dir_all(&scratch);

    assert_eq!(
        without_storage, with_storage,
        "the name index must not depend on the {skipped} Pal storage saves",
    );
}

/// The background jobs are what keep the caches warm and the shared save
/// current. `setup_static_cron` only logs a bad schedule and carries on, so a
/// typo here would silently put the interaction path back on the slow route.
#[test]
fn the_background_jobs_have_valid_schedules() {
    use palworld::cron::{PalworldSaveRefreshCron, PalworldWarmCron};

    let client = std::sync::Arc::new(palworld::client::PalworldClient::new(
        reqwest::Client::new(),
        None,
        None,
        None,
        None,
        std::path::PathBuf::from("palworld_uploads"),
        None,
    ));

    assert!(
        PalworldSaveRefreshCron::cron_job(std::sync::Arc::clone(&client)).is_ok()
    );
    assert!(PalworldWarmCron::cron_job(client).is_ok());
}
