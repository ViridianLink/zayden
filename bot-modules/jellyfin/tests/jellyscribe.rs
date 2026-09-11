//! Jellyscribe's plugin configuration is one document shared by every user, and
//! the write path is a whole-document replace. These pin the two things that
//! makes safe: never clobbering somebody else's account, and never overwriting a
//! username whose stored password belongs to it.

use jellyfin::jellyscribe::{Linked, apply, apply_credentials};
use serde_json::{Value, json};

const ALICE: &str = "aaaaaaaaaaaa4aaaaaaaaaaaaaaaaaaa";
const BOB: &str = "bbbbbbbbbbbb4bbbbbbbbbbbbbbbbbbb";

fn config() -> Value {
    json!({
        "Accounts": [
            {
                "UserJellyfinId": BOB,
                "LetterboxdUsername": "bob",
                "LetterboxdPassword": "hunter2",
                "Enabled": true,
                "SyncFavorites": true,
                "PlaylistName": "Watchlist",
                "AFieldFromANewerPluginVersion": 7
            }
        ],
        "JellyseerrUrl": "https://seer.example",
        "Telemetry": { "Enabled": false }
    })
}

fn accounts(config: &Value) -> &[Value] {
    config["Accounts"].as_array().map_or(&[], Vec::as_slice)
}

#[test]
fn creates_a_disabled_entry_for_a_user_with_no_account() {
    let mut config = config();

    assert_eq!(apply(&mut config, ALICE, "alice"), Linked::Created);

    let added = &accounts(&config)[1];
    assert_eq!(added["UserJellyfinId"], json!(ALICE));
    assert_eq!(added["LetterboxdUsername"], json!("alice"));
    assert_eq!(added["LetterboxdPassword"], json!(""));
    assert_eq!(added["Enabled"], json!(false));
}

#[test]
fn leaves_every_other_account_byte_for_byte() {
    let mut config = config();
    let before = accounts(&config)[0].clone();

    apply(&mut config, ALICE, "alice");

    assert_eq!(accounts(&config)[0], before);
}

#[test]
fn keeps_sibling_configuration_keys() {
    let mut config = config();

    apply(&mut config, ALICE, "alice");

    assert_eq!(config["JellyseerrUrl"], json!("https://seer.example"));
    assert_eq!(config["Telemetry"], json!({ "Enabled": false }));
}

#[test]
fn an_absent_accounts_key_becomes_an_array() {
    let mut config = json!({ "JellyseerrUrl": Value::Null });

    assert_eq!(apply(&mut config, ALICE, "alice"), Linked::Created);
    assert_eq!(accounts(&config).len(), 1);
}

#[test]
fn a_matching_username_is_left_alone() {
    let mut config = config();
    let before = config.clone();

    assert_eq!(apply(&mut config, BOB, "bob"), Linked::Unchanged);
    assert_eq!(config, before);
}

#[test]
fn username_matching_ignores_case() {
    let mut config = config();

    assert_eq!(apply(&mut config, BOB, "BoB"), Linked::Unchanged);
}

#[test]
fn a_different_username_is_a_conflict_not_an_overwrite() {
    let mut config = config();
    let before = config.clone();

    assert_eq!(
        apply(&mut config, BOB, "robert"),
        Linked::Conflict("bob".to_owned())
    );
    assert_eq!(config, before);
}

#[test]
fn a_blank_username_is_filled_in_place() {
    let mut config = json!({
        "Accounts": [{ "UserJellyfinId": ALICE, "LetterboxdUsername": "" }]
    });

    assert_eq!(apply(&mut config, ALICE, "alice"), Linked::Created);

    assert_eq!(accounts(&config).len(), 1);
    assert_eq!(accounts(&config)[0]["LetterboxdUsername"], json!("alice"));
}

#[test]
fn user_ids_match_across_dashed_and_dashless_forms() {
    let dashed = "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa";
    let mut config = json!({
        "Accounts": [{
            "UserJellyfinId": dashed,
            "LetterboxdUsername": "alice"
        }]
    });

    assert_eq!(apply(&mut config, ALICE, "alice"), Linked::Unchanged);
}

#[test]
fn a_second_account_is_not_added_alongside_a_named_one() {
    let mut config = config();

    apply(&mut config, BOB, "robert");

    assert_eq!(accounts(&config).len(), 1);
}

#[test]
fn credentials_create_an_enabled_account() {
    let mut config = config();

    assert!(apply_credentials(&mut config, ALICE, "alice", "s3cret"));

    let added = &accounts(&config)[1];
    assert_eq!(added["UserJellyfinId"], json!(ALICE));
    assert_eq!(added["LetterboxdUsername"], json!("alice"));
    assert_eq!(added["LetterboxdPassword"], json!("s3cret"));
    assert_eq!(added["Enabled"], json!(true));
}

#[test]
fn credentials_update_the_matching_account_in_place() {
    let mut config = config();

    assert!(!apply_credentials(&mut config, BOB, "BOB", "rotated"));

    assert_eq!(accounts(&config).len(), 1);
    assert_eq!(accounts(&config)[0]["LetterboxdPassword"], json!("rotated"));
    assert_eq!(accounts(&config)[0]["LetterboxdUsername"], json!("BOB"));
}

#[test]
fn credentials_keep_the_rest_of_the_matching_account() {
    let mut config = config();

    apply_credentials(&mut config, BOB, "bob", "rotated");

    let account = &accounts(&config)[0];
    assert_eq!(account["PlaylistName"], json!("Watchlist"));
    assert_eq!(account["AFieldFromANewerPluginVersion"], json!(7));
}

#[test]
fn credentials_for_a_new_username_add_a_second_account() {
    let mut config = config();

    assert!(apply_credentials(&mut config, BOB, "robert", "other"));

    assert_eq!(accounts(&config).len(), 2);
    assert_eq!(accounts(&config)[0]["LetterboxdUsername"], json!("bob"));
    assert_eq!(accounts(&config)[0]["LetterboxdPassword"], json!("hunter2"));
}

#[test]
fn credentials_leave_other_users_alone() {
    let mut config = config();
    let before = accounts(&config)[0].clone();

    apply_credentials(&mut config, ALICE, "alice", "s3cret");

    assert_eq!(accounts(&config)[0], before);
}

#[test]
fn a_created_account_carries_the_id_form_the_plugin_matches_on() {
    let dashed = "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa";
    let mut config = json!({ "Accounts": [] });

    apply_credentials(&mut config, dashed, "alice", "s3cret");

    assert_eq!(accounts(&config)[0]["UserJellyfinId"], json!(ALICE));
}
