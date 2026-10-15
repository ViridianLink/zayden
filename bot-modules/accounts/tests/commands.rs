use accounts::commands::{LinkCommand, UnlinkCommand};
use serde_json::Value;

fn names(command: &Value) -> Vec<&str> {
    command.get("options").and_then(Value::as_array).map_or_else(
        Vec::new,
        |options| {
            options
                .iter()
                .filter_map(|option| option.get("name").and_then(Value::as_str))
                .collect()
        },
    )
}

#[test]
fn link_registers_steam_and_status() {
    let command = serde_json::to_value(LinkCommand::register()).unwrap();

    assert_eq!(command.get("name").and_then(Value::as_str), Some("link"));
    assert_eq!(names(&command), ["steam", "status"]);
}

#[test]
fn unlink_registers_steam() {
    let command = serde_json::to_value(UnlinkCommand::register()).unwrap();

    assert_eq!(command.get("name").and_then(Value::as_str), Some("unlink"));
    assert_eq!(names(&command), ["steam"]);
}
