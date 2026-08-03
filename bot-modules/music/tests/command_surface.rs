//! Pins the shape of `/music` against Discord's hard limits.
//!
//! Discord rejects a command with more than 25 options, and the rejection
//! happens at `set_commands` time on `GUILD_CREATE` — not at compile time — so
//! without this test an over-cap command silently fails to register in every
//! guild. `/music` sat at exactly 25 before `radio` was added, which is why
//! `forward`/`rewind` and `removedupes`/`cleanup` were collapsed.

use music::Genre;
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

/// The options of a subcommand nested inside a subcommand *group*, e.g. the `genre`
/// option of `/music radio play`. `option_names` only descends one level.
fn nested_options(group: &str, subcommand: &str) -> Vec<Value> {
    let command =
        serde_json::to_value(music::Command::register()).unwrap_or(Value::Null);

    command
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|opt| opt.get("name").and_then(Value::as_str) == Some(group))
        .and_then(|opt| opt.get("options"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|opt| opt.get("name").and_then(Value::as_str) == Some(subcommand))
        .and_then(|opt| opt.get("options"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

#[test]
fn radio_is_registered_as_a_group_with_its_two_subcommands() {
    assert!(
        subcommand_names().iter().any(|n| n == "radio"),
        "the radio group disappeared from /music",
    );

    let mut sub = option_names("radio");
    sub.sort();
    assert_eq!(sub, ["play", "stop"]);
}

#[test]
fn radio_play_offers_the_genre_catalogue_as_static_choices() {
    let options = nested_options("radio", "play");

    assert_eq!(options.len(), 1, "`radio play` takes exactly one option");

    let Some(genre) = options.first() else {
        panic!("failed to read the genre option");
    };

    assert_eq!(genre.get("name").and_then(Value::as_str), Some("genre"));
    assert_eq!(genre.get("required").and_then(Value::as_bool), Some(true));
    // The catalogue is fixed, so autocomplete would be dead weight — and Discord
    // rejects an option that sets both `choices` and `autocomplete`.
    assert_ne!(genre.get("autocomplete").and_then(Value::as_bool), Some(true));

    let choices: Vec<(&str, &str)> = genre
        .get("choices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|choice| {
            Some((
                choice.get("name").and_then(Value::as_str)?,
                choice.get("value").and_then(Value::as_str)?,
            ))
        })
        .collect();

    let expected: Vec<(&str, &str)> =
        Genre::ALL.iter().map(|g| (g.label(), g.value())).collect();

    // Discord caps a string option at 25 choices and rejects the whole command past
    // that, in every guild, at registration time.
    assert!(
        choices.len() <= 25,
        "`radio play` has {} choices; Discord's cap is 25",
        choices.len(),
    );
    assert_eq!(choices, expected, "the choices must mirror `Genre::ALL` in order");
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
