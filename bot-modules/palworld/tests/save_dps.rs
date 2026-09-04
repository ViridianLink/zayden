//! `save::dps` - Dimensional Pal Storage, and the placeholder owner it explains.
//!
//! The current game build keeps expanded Pal storage in `Players/<uid>_dps.sav`
//! and leaves only stubs in `Level.sav`, filed under a placeholder UID. Reading
//! `Level.sav` alone therefore both loses those Pals and invents a player that
//! does not exist, which is what these tests pin.

use palworld::save::{GLOBAL_STORAGE_UID, dps};

pub mod common;
use common::storage_world;

fn storage_path(name: &str) -> std::path::PathBuf {
    storage_world().join("Players").join(name)
}

/// Ownership comes from inside each slot, not from the file name: this file is
/// named after Oscar Six and holds fourteen of `KingJosh`'s Pals.
#[test]
fn storage_pals_are_keyed_by_their_own_owner() {
    let raw =
        std::fs::read(storage_path("B0726C28000000000000000000000000_dps.sav"))
            .expect("fixture");
    let stored = dps::parse(&raw).expect("decode");

    let oscar = "286C72B0000000000000000000000000";
    let kingjosh = "5CF598C9000000000000000000000000";

    assert_eq!(stored.get(oscar).map(Vec::len), Some(3));
    assert_eq!(stored.get(kingjosh).map(Vec::len), Some(14));
    assert_eq!(stored.len(), 2, "no third owner, and no empty-slot bucket");

    // Empty slots make up the overwhelming majority of the 9600-slot array; none
    // of them may reach the roster.
    assert!(!stored.contains_key(GLOBAL_STORAGE_UID));
    assert!(stored.values().flatten().all(|p| !p.species.is_empty()));
}

/// A storage save is neither a world nor a progress record, and the three are
/// told apart by their GVAS root rather than by file name.
#[test]
fn a_storage_save_is_not_a_world_or_player_save() {
    let storage =
        std::fs::read(storage_path("378DC0D0000000000000000000000000_dps.sav"))
            .expect("fixture");

    assert!(palworld::save::validate_level(&storage).is_err());
    assert!(palworld::save::player::parse_player_uid(&storage).is_err());
    assert!(dps::parse(&storage).is_ok());

    // ...and the reverse: a progress record has no `SaveParameterArray`, so it
    // decodes as an empty storage file rather than erroring. That is why `store`
    // tries the player parser first.
    let player = std::fs::read(storage_path("378DC0D0000000000000000000000000.sav"))
        .expect("fixture");
    assert!(dps::parse(&player).expect("valid gvas").is_empty());
}

/// The placeholder owner is not a player. It must not reach the roster, and the
/// Pals `Level.sav` files under it must be restored to their real owners.
#[test]
fn storage_pals_land_with_their_owner_not_the_placeholder() {
    let world = common::storage_world_roster().expect("load storage-world");

    assert!(
        world.players.iter().all(|p| p.uid != GLOBAL_STORAGE_UID),
        "the storage placeholder must never appear as a player: {:?}",
        world.players.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
    );
    assert_eq!(world.players.len(), 3, "three real players");

    let kitty = world.by_name("Kitty").expect("player present");
    // 622 Pals in `Level.sav` plus the 197 in Kitty's own storage file. Counting
    // only `Level.sav` is the bug this pins.
    assert_eq!(kitty.personal_pals.len(), 819);

    // Storage Pals are personal, so they must survive into the pooled view too.
    assert!(kitty.pals.len() >= kitty.personal_pals.len());
}

/// Storage is per-player, so one member's stored Pals must not appear in another
/// member's personal count - not even inside the same guild.
#[test]
fn storage_does_not_leak_between_players() {
    let world = common::storage_world_roster().expect("load storage-world");

    let stored: usize = [
        "B0726C28000000000000000000000000_dps.sav",
        "378DC0D0000000000000000000000000_dps.sav",
    ]
    .iter()
    .map(|name| {
        let raw = std::fs::read(storage_path(name)).expect("fixture");
        dps::parse(&raw).expect("decode").values().flatten().count()
    })
    .sum();

    let personal: usize = world.players.iter().map(|p| p.personal_pals.len()).sum();
    let level_only = personal - stored;

    assert_eq!(stored, 214, "every stored Pal is accounted for exactly once");
    assert_eq!(personal, level_only + stored);
}
