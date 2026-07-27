//! Regression coverage for the endgame sheet's re-issue name markers.
//!
//! A weapon that returns in a later raid/event is listed in the endgame-analysis
//! sheet under its base name plus a marker naming the re-issue, on a second line
//! (`"Reckless Oracle\nPantheon version"`) or parenthesised
//! (`"Hung Jury SR4 (BRAVE version)"`). The Bungie manifest only knows the base
//! name, so the marker must come off before the lookup — otherwise
//! `parse_weapon_data` logs `Missing item: <name>` and falls back to a default,
//! icon- and hash-less item.
//!
//! This used to be a hard-coded list of the markers seen so far (`BRAVE`,
//! `RotN`, plus a one-off exact-match arm for `"Long Arm\nRotn version"` because
//! its capitalisation differed). Pantheon then shipped and broke a fresh batch
//! of weapons — Reckless Oracle, Zaouli's Bane, Chattering Bone — in production.
//! Matching the marker's *shape* fixes the whole class, so the next re-issue
//! needs no code change.

use destiny2::endgame_analysis::sheet::{WeaponBuilder, strip_reissue_annotation};

#[test]
fn pantheon_marker_is_stripped() {
    // The exact production failures.
    assert_eq!(
        strip_reissue_annotation("Reckless Oracle\nPantheon version"),
        "Reckless Oracle"
    );
    assert_eq!(
        strip_reissue_annotation("Zaouli's Bane\nPantheon version"),
        "Zaouli's Bane"
    );
    assert_eq!(
        strip_reissue_annotation("Chattering Bone\nPantheon version"),
        "Chattering Bone"
    );
}

#[test]
fn previously_hardcoded_markers_still_strip() {
    assert_eq!(
        strip_reissue_annotation("Hung Jury SR4\nBRAVE version"),
        "Hung Jury SR4"
    );
    assert_eq!(
        strip_reissue_annotation("Hung Jury SR4 (BRAVE version)"),
        "Hung Jury SR4"
    );
    assert_eq!(
        strip_reissue_annotation("Midnight Coup\nRotN version"),
        "Midnight Coup"
    );
    // The lowercase-n spelling that needed its own exact-match arm before.
    assert_eq!(strip_reissue_annotation("Long Arm\nRotn version"), "Long Arm");
}

#[test]
fn unmarked_names_pass_through_untouched() {
    assert_eq!(strip_reissue_annotation("Fatebringer"), "Fatebringer");
    assert_eq!(strip_reissue_annotation("  Fatebringer  "), "Fatebringer");
    assert_eq!(strip_reissue_annotation("IKELOS_SMG_v1.0.3"), "IKELOS_SMG_v1.0.3");
}

#[test]
fn only_version_shaped_markers_are_stripped() {
    // A second line that is not a re-issue marker must survive — dropping it
    // would silently rename a weapon.
    let note = "Some Weapon\nsee notes below";
    assert_eq!(strip_reissue_annotation(note), note);

    let paren = "Some Weapon (Adept)";
    assert_eq!(strip_reissue_annotation(paren), paren);
}

#[test]
fn a_bare_marker_never_empties_the_name() {
    // Degenerate rows must not collapse to "" — an empty name would match
    // nothing in the manifest while still looking like a successful parse.
    // A name that is *only* a marker keeps the marker text (it then fails the
    // manifest lookup like any other unknown name, which is the safe outcome).
    assert_eq!(strip_reissue_annotation("\nPantheon version"), "Pantheon version");
    assert_eq!(strip_reissue_annotation("   "), "");

    // "version" alone is too short to be a marker, so the name survives whole
    // rather than being truncated to "Weapon".
    assert_eq!(strip_reissue_annotation("Weapon\nversion"), "Weapon\nversion");

    for name in ["Fatebringer", "A\nPantheon version", "A (BRAVE version)"] {
        assert!(
            !strip_reissue_annotation(name).is_empty(),
            "stripping must never empty a non-blank name: {name:?}"
        );
    }
}

#[test]
fn marker_is_stripped_before_the_manifest_spelling_map() {
    // `WeaponBuilder::new` corrects sheet spellings that differ from the
    // manifest. A marker on one of those names used to defeat the exact match,
    // so the correction silently stopped applying.
    let builder =
        WeaponBuilder::new("Song of Ir Yut\nPantheon version", "Machine Gun");
    assert_eq!(builder.name, "Song of Ir Yût");

    let plain = WeaponBuilder::new("Song of Ir Yut", "Machine Gun");
    assert_eq!(plain.name, "Song of Ir Yût");
}
