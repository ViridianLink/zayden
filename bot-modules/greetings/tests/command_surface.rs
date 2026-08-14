//! Pins the shape of `/good` to what the runtime dispatch expects.
//!
//! `run` resolves the invoked subcommand by feeding its name straight to
//! [`GreetingKind::parse`], so the registered subcommand names and the parser
//! are one contract split across two files. If they drift, every invocation
//! fails at runtime with "Unknown greeting type" — Discord accepts the
//! registration either way, so nothing else catches it.

use greetings::{GreetingKind, register};
use serde_json::Value;

/// Total by construction: the workspace lints deny `expect`/indexing outside
/// `#[test]` fns, and an unexpected shape yields an empty list which the
/// assertions below catch.
fn subcommand_names() -> Vec<String> {
    let command = serde_json::to_value(register()).unwrap_or(Value::Null);

    command
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|opt| opt.get("name").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

#[test]
fn registers_as_good_with_a_subcommand_per_kind() {
    let command = serde_json::to_value(register()).unwrap_or(Value::Null);

    assert_eq!(
        command.get("name").and_then(Value::as_str),
        Some("good"),
        "the greetings command must register as /good"
    );

    assert_eq!(
        subcommand_names(),
        vec!["morning".to_string(), "night".to_string()],
        "/good must expose exactly the morning and night subcommands"
    );
}

#[test]
fn every_subcommand_name_parses_back_to_its_kind() {
    for name in subcommand_names() {
        let kind = GreetingKind::parse(&name)
            .unwrap_or_else(|_| panic!("/good {name} has no matching kind"));

        assert_eq!(
            kind.subcommand_name(),
            name,
            "kind {kind} does not round-trip through its subcommand name"
        );
    }
}

#[test]
fn each_kind_takes_an_optional_user() {
    let command = serde_json::to_value(register()).unwrap_or(Value::Null);

    let subcommands =
        command.get("options").and_then(Value::as_array).into_iter().flatten();

    for sub in subcommands {
        let name = sub.get("name").and_then(Value::as_str).unwrap_or_default();

        let options = sub
            .get("options")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!(options.len(), 1, "/good {name} should take one option");

        let option = options.first().copied().unwrap_or(&Value::Null);

        assert_eq!(
            option.get("name").and_then(Value::as_str),
            Some("user"),
            "/good {name} should take a `user` option"
        );
        assert_ne!(
            option.get("required").and_then(Value::as_bool),
            Some(true),
            "/good {name} should default to greeting the invoker"
        );
    }
}
