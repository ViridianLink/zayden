//! Pins the `/family` parent command against its dispatcher.
//!
//! The subcommand list is declared in one place (`Command::register`) and
//! matched on in another (`Command::run`). Nothing in the type system ties the
//! two together, so a rename or a typo in either half silently produces a
//! subcommand Discord advertises but the bot rejects -- or a match arm that can
//! never fire. These tests read the built definition back out and assert the
//! shape the dispatcher expects.

use family::commands::Command;
use serde_json::Value;

/// Every subcommand `Command::run` dispatches on, in registration order.
const SUBCOMMANDS: [&str; 12] = [
    "marry",
    "divorce",
    "adopt",
    "tree",
    "relationship",
    "children",
    "parents",
    "partner",
    "siblings",
    "block",
    "unblock",
    "reset",
];

/// Discord's option-type discriminant for `SUB_COMMAND`.
const SUB_COMMAND: u64 = 1;

/// Discord's per-command option cap.
const MAX_OPTIONS: usize = 25;

// A macro rather than a function: `clippy.toml` allows `.expect()` only inside a
// `#[test]` item, so a free helper fn using it trips `expect_used` under the
// workspace `-D warnings` gate.
macro_rules! definition {
    () => {
        serde_json::to_value(Command::register()).expect("definition serialises")
    };
}

macro_rules! options {
    ($value:expr) => {
        $value.get("options").and_then(Value::as_array).expect("options array")
    };
}

fn name_of(value: &Value) -> &str {
    value.get("name").and_then(Value::as_str).unwrap_or_default()
}

#[test]
fn the_parent_command_is_named_family() {
    let definition = definition!();

    assert_eq!(name_of(&definition), "family");
}

#[test]
fn every_dispatched_subcommand_is_registered() {
    let definition = definition!();
    let options = options!(definition);

    let names: Vec<&str> = options.iter().map(name_of).collect();

    assert_eq!(names, SUBCOMMANDS);
}

#[test]
fn every_option_is_a_subcommand() {
    let definition = definition!();
    let options = options!(definition);

    for option in options {
        assert_eq!(
            option.get("type").and_then(Value::as_u64),
            Some(SUB_COMMAND),
            "`{}` is not a subcommand",
            name_of(option)
        );
    }
}

/// Discord rejects a subcommand whose required options follow optional ones,
/// and rejects the whole registration with it -- taking every other subcommand
/// down too.
#[test]
fn required_options_come_before_optional_ones() {
    let definition = definition!();
    let options = options!(definition);

    for option in options {
        let Some(sub_options) = option.get("options").and_then(Value::as_array)
        else {
            continue;
        };

        let mut seen_optional = false;
        for sub_option in sub_options {
            let required =
                sub_option.get("required").and_then(Value::as_bool).unwrap_or(false);

            assert!(
                !(required && seen_optional),
                "{}: required option `{}` follows an optional one",
                name_of(option),
                name_of(sub_option)
            );

            seen_optional |= !required;
        }
    }
}

#[test]
fn the_command_fits_discords_option_cap() {
    let definition = definition!();
    let options = options!(definition);

    assert!(options.len() <= MAX_OPTIONS, "{} options", options.len());
}
