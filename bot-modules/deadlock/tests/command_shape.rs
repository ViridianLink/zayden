use deadlock::commands::Command;
use serde_json::Value;
use zayden_core::limits::command::OPTIONS as MAX_OPTIONS;

const SUBCOMMANDS: [&str; 5] =
    ["build", "abilities", "counter", "matchup", "summaries"];

const SUB_COMMAND: u64 = 1;
const STRING: u64 = 3;

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
fn the_parent_command_is_named_deadlock() {
    let definition = definition!();

    assert_eq!(name_of(&definition), "deadlock");
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

#[test]
fn required_options_come_before_optional_ones() {
    let definition = definition!();
    let options = options!(definition);

    for option in options {
        let sub_options = options!(option);

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
fn hero_and_rank_options_offer_autocomplete() {
    let definition = definition!();
    let options = options!(definition);

    for option in options {
        for sub_option in options!(option) {
            if sub_option.get("type").and_then(Value::as_u64) != Some(STRING) {
                continue;
            }

            let autocomplete = sub_option
                .get("autocomplete")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            assert!(
                autocomplete,
                "{}: option `{}` has no autocomplete",
                name_of(option),
                name_of(sub_option)
            );
        }
    }
}

#[test]
fn the_command_fits_discords_option_cap() {
    let definition = definition!();
    let options = options!(definition);

    assert!(options.len() <= MAX_OPTIONS, "{} options", options.len());
    for option in options {
        let sub_options = options!(option);
        assert!(
            sub_options.len() <= MAX_OPTIONS,
            "{}: {} options",
            name_of(option),
            sub_options.len()
        );
    }
}
