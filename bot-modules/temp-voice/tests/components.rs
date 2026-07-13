//! Panel build + custom-id routing checks for the temp-voice control panel.
//!
//! The resolver's live branching (interaction channel → managed temp channel;
//! else the clicker's connected channel; else "no active channel") needs a
//! Postgres pool and is exercised end-to-end; here we lock down the pure,
//! DB-free contract: the panel wires exactly the expected custom ids, and the
//! routing constants stay unique and correctly namespaced.

use serde_json::Value;
use temp_voice::components::{
    self,
    build_panel,
    PRIVACIES,
    REGIONS,
};

/// Every button custom id the panel is expected to expose, across all rows.
const EXPECTED_BUTTONS: &[&str] = &[
    components::RENAME,
    components::LIMIT,
    components::BITRATE,
    components::PASSWORD,
    components::PRIVACY,
    components::REGION,
    components::CLAIM,
    components::TRANSFER,
    components::TRUST,
    components::KICK,
    components::DELETE,
];

/// The full set of routed ids: panel buttons plus the second-step select menus.
const ALL_IDS: &[&str] = &[
    components::RENAME,
    components::LIMIT,
    components::BITRATE,
    components::PASSWORD,
    components::PRIVACY,
    components::REGION,
    components::CLAIM,
    components::TRANSFER,
    components::TRUST,
    components::KICK,
    components::DELETE,
    components::PRIVACY_MENU,
    components::REGION_MENU,
    components::TRANSFER_MENU,
    components::TRUST_MENU,
    components::KICK_MENU,
];

/// Collect the `custom_id` of every button in the serialized panel. Returns an
/// empty vec if serialization or the expected shape ever breaks, which the
/// structural assertions below then surface as a mismatch.
fn panel_button_ids() -> Vec<String> {
    let panel = build_panel();
    let value = serde_json::to_value(&panel).unwrap_or_default();

    value
        .as_array()
        .map(|rows| {
            rows.iter()
                .flat_map(|row| {
                    row.get("components")
                        .and_then(Value::as_array)
                        .map(|comps| {
                            comps
                                .iter()
                                .filter_map(|c| {
                                    c.get("custom_id")
                                        .and_then(Value::as_str)
                                        .map(String::from)
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn panel_has_three_rows() {
    assert_eq!(build_panel().len(), 3);
}

#[test]
fn panel_exposes_exactly_the_expected_buttons() {
    let mut ids = panel_button_ids();
    ids.sort();

    let mut expected =
        EXPECTED_BUTTONS.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    expected.sort();

    assert_eq!(ids, expected);
}

#[test]
fn every_button_id_is_voice_namespaced() {
    for id in panel_button_ids() {
        assert!(id.starts_with("voice_"), "custom id `{id}` is not namespaced");
    }
}

#[test]
fn routing_ids_are_unique() {
    let mut seen = ALL_IDS.to_vec();
    seen.sort_unstable();
    let unique_len = {
        let mut deduped = seen.clone();
        deduped.dedup();
        deduped.len()
    };

    assert_eq!(seen.len(), unique_len, "duplicate routing custom id present");
}

#[test]
fn button_and_menu_ids_never_collide() {
    // Every select-menu id must differ from its opener button id so the two
    // exact-match component routes stay distinct.
    for menu in [
        components::PRIVACY_MENU,
        components::REGION_MENU,
        components::TRANSFER_MENU,
        components::TRUST_MENU,
        components::KICK_MENU,
    ] {
        assert!(EXPECTED_BUTTONS.iter().all(|button| *button != menu));
    }
}

#[test]
fn privacy_values_match_action_arms() {
    // The privacy action dispatches on exactly these values.
    let values = PRIVACIES.iter().map(|(_, v)| *v).collect::<Vec<_>>();
    assert_eq!(values, ["open", "spectator", "lock", "invisible"]);
}

#[test]
fn region_options_lead_with_automatic() {
    // "automatic" is mapped back to `None` (clear region) by the region menu.
    assert_eq!(REGIONS.first().map(|(_, v)| *v), Some("automatic"));
    assert!(REGIONS.len() > 1, "region list should offer concrete regions too");
}
