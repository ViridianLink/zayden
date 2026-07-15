//! `save::palmap::resolve_species` - internal `CharacterID` → `Pal.key`.

use palworld::model::Pal;
use palworld::save::palmap::resolve_species;

fn pal(key: &str, name: &str) -> Pal {
    Pal { key: key.to_string(), name: name.to_string(), ..Pal::default() }
}

fn pals() -> Vec<Pal> {
    vec![
        pal("SheepBall", "Lamball"),
        pal("Bastet_Ice", "Sibelyx"),
        pal("PlantSlime_Flower", "Bristla"),
    ]
}

#[test]
fn direct_key_match() {
    assert_eq!(resolve_species("SheepBall", &pals()).as_deref(), Some("SheepBall"));
}

#[test]
fn boss_prefix_is_stripped() {
    assert_eq!(
        resolve_species("BOSS_SheepBall", &pals()).as_deref(),
        Some("SheepBall")
    );
}

#[test]
fn gym_prefix_is_stripped() {
    assert_eq!(
        resolve_species("GYM_SheepBall", &pals()).as_deref(),
        Some("SheepBall")
    );
}

#[test]
fn matching_is_case_insensitive() {
    assert_eq!(resolve_species("sheepball", &pals()).as_deref(), Some("SheepBall"));
}

#[test]
fn mixed_case_boss_prefix_is_stripped() {
    // The real save emits both `BOSS_` and `Boss_`; the prefix match is
    // case-insensitive so alpha bosses like `Boss_Anubis` still resolve.
    assert_eq!(
        resolve_species("Boss_SheepBall", &pals()).as_deref(),
        Some("SheepBall")
    );
}

#[test]
fn variant_suffix_is_preserved() {
    // Element/variant suffixes are part of the PalCalc key, so they must not be
    // stripped along with the BOSS_ prefix.
    assert_eq!(
        resolve_species("BOSS_Bastet_Ice", &pals()).as_deref(),
        Some("Bastet_Ice")
    );
}

#[test]
fn falls_back_to_display_name() {
    assert_eq!(resolve_species("Lamball", &pals()).as_deref(), Some("SheepBall"));
}

#[test]
fn unknown_codename_resolves_to_none() {
    // Human NPC records (BOSS_Male_Soldier04, …) have no Pal and are skipped.
    assert_eq!(resolve_species("BOSS_Male_Soldier04", &pals()), None);
}
