//! End-to-end `load_world` validation against the real world save.
//!
//! Gated on `PALWORLD_SAVE=1` (mirroring the `PALWORLD_LIVE=1` gate in
//! `tests/live.rs`) so `cargo test` stays green in CI and offline. It parses the
//! real `Level.sav` at `056C426C…/`, then confirms the two things the temporary
//! `/palworld roster` harness existed to eyeball:
//!   1. at least one player has a non-empty, gendered roster, and
//!   2. every owned `CharacterID` codename resolves to a `Pal.key` - proving the
//!      `palmap` override table is complete for this save.
//!
//! The Pal list is fetched live from `PalCalc` (keys are internal names, which
//! match the save's `CharacterID` base spelling) - the full ~300-pal dex the 15-
//! pal breeding fixture can't cover. Network is acceptable behind the opt-in gate.

use std::collections::BTreeSet;
use std::path::PathBuf;

use palworld::client::PalworldClient;
use palworld::model::Gender;
use palworld::save::load_world;
use palworld::save::palmap::resolve_species;

fn save_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../056C426C55974CFCA115EB695A224F67")
}

fn client() -> PalworldClient {
    PalworldClient::new(
        reqwest::Client::new(),
        None,
        None,
        None,
        None,
        PathBuf::from("palworld_uploads"),
        None,
    )
}

/// Skip unless explicitly enabled *and* the save is decodable on disk.
fn enabled() -> bool {
    if std::env::var("PALWORLD_SAVE").is_err() {
        eprintln!("skipping: set PALWORLD_SAVE=1 to run the real-save fixture test");
        return false;
    }
    match std::fs::read(save_dir().join("Level.sav")) {
        Ok(raw) => match palworld::save::validate_level(&raw) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("skipping: save present but not decodable ({e})");
                false
            },
        },
        Err(e) => {
            eprintln!("skipping: real save not present ({e})");
            false
        },
    }
}

#[test]
fn load_world_yields_gendered_rosters() {
    if !enabled() {
        return;
    }
    let world = load_world(&save_dir()).expect("load_world");

    assert!(!world.players.is_empty(), "world decodes at least one player");

    // At least one player owns pals, and across the world at least one owned pal
    // carries a known (Male/Female) gender - the field the breeding-readiness
    // logic depends on.
    let with_pals = world.players.iter().filter(|p| !p.pals.is_empty()).count();
    assert!(with_pals >= 1, "at least one player has a non-empty roster");

    let gendered = world
        .players
        .iter()
        .flat_map(|p| &p.pals)
        .filter(|owned| owned.gender != Gender::Unknown)
        .count();
    assert!(gendered >= 1, "at least one owned pal has a known gender");

    eprintln!(
        "load_world: {} players, {} with pals, {} gendered pals",
        world.players.len(),
        with_pals,
        gendered
    );
}

/// Codenames that legitimately have no breedable `Pal` and are expected to
/// resolve to `None`: capturable human NPCs (soldiers, hunters, merchants) and
/// `GrassBoss`, a special boss with no entry in `PalCalc`'s breeding DB. The
/// command layer skips these when building a breeding roster.
const NON_PAL_CODENAMES: &[&str] = &[
    "BOSS_Female_People03",
    "BOSS_Male_Soldier04",
    "Believer_CrossBow",
    "GrassBoss",
    "Hunter_Handgun",
    "Hunter_Rifle",
    "PalDealer",
    "RandomEventShop",
];

#[tokio::test]
async fn every_owned_codename_resolves_to_a_pal() {
    if !enabled() {
        return;
    }
    let world = load_world(&save_dir()).expect("load_world");
    let pals = client().pals().await.expect("live pals fetch");
    assert!(!pals.is_empty(), "pal list fetched");

    let codenames: BTreeSet<&str> = world
        .players
        .iter()
        .flat_map(|p| &p.pals)
        .map(|owned| owned.species.as_str())
        .collect();
    assert!(!codenames.is_empty(), "roster carries owned codenames");

    // Every owned codename either resolves to a `Pal.key` or is a known non-pal
    // record. A new unresolved codename means the save added a pal (or an alias)
    // the `palmap` prefix/override table doesn't yet cover.
    let unexpected: Vec<&str> = codenames
        .iter()
        .filter(|c| resolve_species(c, &pals).is_none())
        .filter(|c| !NON_PAL_CODENAMES.contains(c))
        .copied()
        .collect();

    assert!(
        unexpected.is_empty(),
        "unresolved owned CharacterID(s): {unexpected:?} - extend \
         save::palmap (prefix or OVERRIDES) or the NON_PAL_CODENAMES allowlist",
    );
}
