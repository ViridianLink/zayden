//! Pins the shape of `/music` against Discord's hard limits.
//!
//! Discord rejects a command with more than 25 options, and the rejection
//! happens at `set_commands` time on `GUILD_CREATE` — not at compile time — so
//! without this test an over-cap command silently fails to register in every
//! guild. `/music` sat at exactly 25 before `radio` was added, which is why
//! `forward`/`rewind` and `removedupes`/`cleanup` were collapsed.

use serde_json::Value;

/// Total by construction: the workspace lints deny `expect`/indexing outside
/// `#[test]` fns, and an unexpected shape yields an empty list which the
/// assertions below catch.
fn subcommand_names() -> Vec<String> {
    let command =
        serde_json::to_value(music::Command::register()).unwrap_or(Value::Null);

    command
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|opt| opt.get("name").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

fn option_names(subcommand: &str) -> Vec<String> {
    let command =
        serde_json::to_value(music::Command::register()).unwrap_or(Value::Null);

    command
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|opt| opt.get("name").and_then(Value::as_str) == Some(subcommand))
        .and_then(|opt| opt.get("options"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|opt| opt.get("name").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

#[test]
fn music_stays_within_discords_option_cap() {
    let names = subcommand_names();

    assert!(!names.is_empty(), "failed to read the command definition");
    assert!(
        names.len() <= 25,
        "/music has {} options; Discord's cap is 25 and registration would \
         fail for every guild. Collapse a subcommand before adding another. \
         Found: {names:?}",
        names.len(),
    );
}

#[test]
fn radio_is_registered_as_a_group_with_its_three_subcommands() {
    assert!(
        subcommand_names().iter().any(|n| n == "radio"),
        "the radio group disappeared from /music",
    );

    let mut sub = option_names("radio");
    sub.sort();
    assert_eq!(sub, ["list", "play", "stop"]);
}

#[test]
fn the_collapsed_subcommands_are_gone() {
    let names = subcommand_names();

    // Each of these was folded into another subcommand to free a slot. Bringing
    // one back without removing something else pushes /music over the cap.
    for removed in ["forward", "rewind", "removedupes", "cleanup"] {
        assert!(
            !names.iter().any(|n| n == removed),
            "`{removed}` was collapsed into another subcommand; found: {names:?}",
        );
    }
}

#[test]
fn silent_is_registered_with_its_optional_toggle() {
    assert!(
        subcommand_names().iter().any(|n| n == "silent"),
        "`silent` disappeared from /music",
    );

    // Blank means "toggle", so the option must stay optional; a required
    // `enabled` would break the bare `/music silent` form.
    assert_eq!(option_names("silent"), ["enabled"]);
}

#[test]
fn clear_absorbed_the_collapsed_queue_pruning_modes() {
    assert_eq!(option_names("clear"), ["mode"]);
}

#[test]
fn seek_absorbed_relative_seeking() {
    // `forward`/`rewind` are gone, so `seek` must still take its timestamp.
    assert_eq!(option_names("seek"), ["timestamp"]);
}
