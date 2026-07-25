//! Pins which music settings `/music settings` may edit.
//!
//! Per the audit's CC-8 remediation (music.md #3), the *admin setup* fields —
//! DJ role, auto-disconnect delay, now-playing announcements — moved to the web
//! dashboard, which is now their single editor. The *playback behaviour* fields
//! stay in Discord because they are tweaked live as listeners and tracks change.
//!
//! Two editors writing one column is the exact duplication CC-8 flags, so this
//! test fails if a moved option is ever re-added to the slash command (or if a
//! retained one silently disappears).

use serde_json::Value;

fn name_of(value: &Value) -> Option<&str> {
    value.get("name").and_then(Value::as_str)
}

fn sub_options(value: &Value) -> impl Iterator<Item = &Value> {
    value.get("options").and_then(Value::as_array).into_iter().flatten()
}

/// Option names exposed by the `settings` subcommand of `/music`.
///
/// Total by construction — the workspace lints deny `expect`/indexing outside
/// `#[test]` fns. An unexpected shape yields an empty list, which
/// [`settings_subcommand_exposes_exactly_the_retained_options`] catches.
fn settings_option_names() -> Vec<String> {
    let command =
        serde_json::to_value(music::Command::register()).unwrap_or(Value::Null);

    sub_options(&command)
        .find(|opt| name_of(opt) == Some("settings"))
        .into_iter()
        .flat_map(sub_options)
        .filter_map(name_of)
        .map(ToOwned::to_owned)
        .collect()
}

#[test]
fn dashboard_owned_settings_are_not_editable_from_discord() {
    let names = settings_option_names();

    for moved in
        ["dj_role", "clear_dj_role", "auto_disconnect_secs", "announce_now_playing"]
    {
        assert!(
            !names.iter().any(|n| n == moved),
            "`{moved}` is owned by the dashboard \
             (save_music_settings) and must not be settable from Discord too; \
             found options: {names:?}",
        );
    }
}

#[test]
fn live_playback_settings_stay_editable_from_discord() {
    let names = settings_option_names();

    for kept in ["default_volume", "stay_connected", "autoplay"] {
        assert!(
            names.iter().any(|n| n == kept),
            "`{kept}` is tweaked while music plays and must stay on \
             /music settings; found options: {names:?}",
        );
    }
}

#[test]
fn settings_subcommand_exposes_exactly_the_retained_options() {
    let mut names = settings_option_names();
    names.sort();

    assert_eq!(names, ["autoplay", "default_volume", "stay_connected"]);
}
