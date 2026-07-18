//! Behaviour locked by the endgame-weapon DB layer:
//! - `Affinity` parses the six real elements and rejects everything else
//!   (empty/absent affinity is represented as `NULL`, handled by the builder).
//! - `Frame` is persisted as its `Display` string, so `FromStr` -> `Display` must be
//!   stable for the round trip through the database to be lossless.
//! - `TierLabel` maps to the `destiny2_tier` Postgres enum; its `Display` surface is
//!   what commands render.

use std::str::FromStr;

use destiny2::db::endgame::is_safe_replace;
use destiny2::endgame_analysis::sheet::{Affinity, Frame, TierLabel};

#[test]
fn affinity_parses_the_six_elements() {
    for (input, expected) in [
        ("Kinetic", "Kinetic"),
        ("Arc", "Arc"),
        ("Void", "Void"),
        ("Solar", "Solar"),
        ("Stasis", "Stasis"),
        ("Strand", "Strand"),
    ] {
        let affinity = Affinity::from_str(input).expect("known affinity");
        assert_eq!(affinity.to_string(), expected);
    }
}

#[test]
fn affinity_rejects_empty_and_unknown() {
    assert!(Affinity::from_str("").is_err());
    assert!(Affinity::from_str("None").is_err());
    assert!(Affinity::from_str("kinetic").is_err());
}

#[test]
fn frame_display_round_trips_from_sheet_string() {
    // The upstream sheet now emits the heat frames without the RPM qualifier
    // (bare `Balanced` / `Dynamic`); these must parse and Display as themselves.
    // (Regression: prod logged `Failed to parse: 'Dynamic'` / `'Balanced'`,
    // which silently dropped those weapons from the tierlist on refresh.)
    let balanced = Frame::from_str("Balanced").expect("bare heat frame");
    assert_eq!(balanced.to_string(), "Balanced");

    let dynamic = Frame::from_str("Dynamic").expect("bare heat frame");
    assert_eq!(dynamic.to_string(), "Dynamic");

    // The legacy `(NNNRPM)` forms are still accepted for rows the sheet has not
    // yet reformatted, collapsing onto the same bare Display.
    assert_eq!(
        Frame::from_str("Dynamic (180RPM)").expect("legacy heat frame").to_string(),
        "Dynamic"
    );
    assert_eq!(
        Frame::from_str("Balanced (260RPM)").expect("legacy heat frame").to_string(),
        "Balanced"
    );

    // Simple frames Display exactly as parsed.
    let precision = Frame::from_str("Precision").expect("known frame");
    assert_eq!(precision.to_string(), "Precision");
}

#[test]
fn frame_rejects_unknown() {
    assert!(Frame::from_str("Definitely Not A Frame").is_err());
}

/// Destiny 2 has shipped its final update, so the endgame sheet's frame
/// vocabulary is closed. This fixture is every distinct frame string observed
/// across all weapon tabs of the live sheet (scanned 2026-07-18); since the
/// vocabulary can no longer grow, it is authoritative. Each string must parse —
/// a parse failure silently drops the weapon and the next refresh deletes it
/// (the DS-3 regression) — and each stored `Display` form must re-parse so the
/// value survives the round trip through the database.
#[test]
fn frame_parses_every_string_in_the_final_sheet() {
    const SHEET_FRAMES: &[&str] = &[
        "Adaptive",
        "Adaptive Burst",
        "Aggressive",
        "Aggressive Burst",
        "Area Denial",
        "Balanced",
        "Caster",
        "Compressed Wave",
        "Disruption",
        "Double Fire",
        "Dynamic",
        "Dynamic (180RPM)",
        "Heavy Burst",
        "High-Impact",
        "High-Impact Longbow",
        "Legacy PR-55",
        "Lightweight",
        "MIDA Synergy",
        "Micro-Missile",
        "Pinpoint Slug",
        "Precision",
        "Rapid",
        "Rapid Slug",
        "Shot Package",
        "Spread Shot",
        "Support",
        "Together Forever",
        "Vortex",
        "Wave",
    ];

    for input in SHEET_FRAMES {
        let frame = Frame::from_str(input)
            .unwrap_or_else(|()| panic!("final-sheet frame '{input}' must parse"));
        let stored = frame.to_string();
        Frame::from_str(&stored).unwrap_or_else(|()| {
            panic!("stored frame '{stored}' (from '{input}') must re-parse")
        });
    }
}

#[test]
fn replace_guard_seeds_empty_catalog() {
    // An empty table must accept any parse result so the catalog can bootstrap.
    assert!(is_safe_replace(0, 0));
    assert!(is_safe_replace(0, 5));
}

#[test]
fn replace_guard_blocks_catalog_erasure() {
    // Parser/upstream drift that drops everything (or most rows) must not wipe
    // a populated catalog — this is the destructive `TRUNCATE`-replace guard.
    assert!(!is_safe_replace(800, 0), "empty parse must not wipe the catalog");
    assert!(!is_safe_replace(800, 399), "losing >half must be refused");
}

#[test]
fn replace_guard_allows_normal_churn() {
    // Ordinary seasonal churn (small additions/removals) proceeds.
    assert!(is_safe_replace(800, 800));
    assert!(is_safe_replace(800, 810));
    assert!(is_safe_replace(800, 400), "exactly half is still accepted");
}

#[test]
fn tier_label_parses_and_displays() {
    for (input, label) in [
        ("S", "S"),
        ("A", "A"),
        ("B", "B"),
        ("C", "C"),
        ("D", "D"),
        ("E", "E"),
        ("F", "F"),
    ] {
        let tier = TierLabel::from_str(input).expect("known tier");
        assert_eq!(tier.to_string(), label);
    }

    // Unknown parses fall back to the caller; `None` renders explicitly.
    assert!(TierLabel::from_str("None").is_err());
    assert_eq!(TierLabel::None.to_string(), "None");
}
