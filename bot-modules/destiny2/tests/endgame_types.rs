//! Behaviour locked by the endgame-weapon DB layer:
//! - `Affinity` parses the six real elements and rejects everything else
//!   (empty/absent affinity is represented as `NULL`, handled by the builder).
//! - `Frame` is persisted as its `Display` string, so `FromStr` -> `Display` must be
//!   stable for the round trip through the database to be lossless.
//! - `TierLabel` maps to the `destiny2_tier` Postgres enum; its `Display` surface is
//!   what commands render.

use std::str::FromStr;

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
    // The stored value is the Display form; these are the cases where the
    // sheet input and Display output differ, so they must stay in lockstep.
    let balanced = Frame::from_str("Balanced (260RPM)").expect("known frame");
    assert_eq!(balanced.to_string(), "Balanced Heat (260RPM)");

    let dynamic = Frame::from_str("Dynamic (140RPM)").expect("known frame");
    assert_eq!(dynamic.to_string(), "Dynamic Heat (140RPM)");

    // Simple frames Display exactly as parsed.
    let precision = Frame::from_str("Precision").expect("known frame");
    assert_eq!(precision.to_string(), "Precision");
}

#[test]
fn frame_rejects_unknown() {
    assert!(Frame::from_str("Definitely Not A Frame").is_err());
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
