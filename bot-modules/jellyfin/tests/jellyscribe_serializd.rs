//! The Serializd half of Jellyscribe's configuration. It shares the Letterboxd
//! document and its whole-document replace, so the same guarantees apply, and
//! neither account list may disturb the other.

use jellyfin::jellyscribe::serializd::{Account, apply_credentials, find};
use serde_json::{Value, json};

const ALICE: &str = "aaaaaaaaaaaa4aaaaaaaaaaaaaaaaaaa";
const BOB: &str = "bbbbbbbbbbbb4bbbbbbbbbbbbbbbbbbb";

fn config() -> Value {
    json!({
        "Accounts": [
            {
                "UserJellyfinId": ALICE,
                "LetterboxdUsername": "alice",
                "LetterboxdPassword": "lb-secret",
                "Enabled": true
            }
        ],
        "SerializdAccounts": [
            {
                "UserJellyfinId": BOB,
                "Email": "bob@example.com",
                "Password": "hunter2",
                "SerializdUsername": "bobwatches",
                "Enabled": true,
                "SyncWatchlist": true,
                "WatchlistName": "Bob's shows",
                "AFieldFromANewerPluginVersion": 7
            }
        ],
        "JellyseerrUrl": "https://seer.example"
    })
}

fn accounts(config: &Value) -> &[Value] {
    config["SerializdAccounts"].as_array().map_or(&[], Vec::as_slice)
}

#[test]
fn credentials_create_an_enabled_account() {
    let mut config = config();

    assert!(apply_credentials(
        &mut config,
        ALICE,
        "alice@example.com",
        "s3cret",
        Some("alicewatches")
    ));

    let added = &accounts(&config)[1];
    assert_eq!(added["UserJellyfinId"], json!(ALICE));
    assert_eq!(added["Email"], json!("alice@example.com"));
    assert_eq!(added["Password"], json!("s3cret"));
    assert_eq!(added["SerializdUsername"], json!("alicewatches"));
    assert_eq!(added["Enabled"], json!(true));
}

#[test]
fn a_created_account_carries_the_configured_defaults() {
    let mut config = config();

    apply_credentials(&mut config, ALICE, "alice@example.com", "s3cret", None);

    let added = &accounts(&config)[1];
    for on in [
        "SyncFavorites",
        "IsPrimary",
        "SyncWatchlist",
        "AutoRequestWatchlist",
        "SkipPreviouslySynced",
        "EnableDiaryImport",
    ] {
        assert_eq!(added[on], json!(true), "{on} should default on");
    }

    for off in [
        "EnableDateFilter",
        "BackfillAvailableRequests",
        "MirrorJellyseerrWatchlist",
        "StopOnFailure",
    ] {
        assert_eq!(added[off], json!(false), "{off} should default off");
    }
}

#[test]
fn an_unknown_username_is_not_written() {
    let mut config = config();

    apply_credentials(&mut config, ALICE, "alice@example.com", "s3cret", None);

    assert!(accounts(&config)[1].get("SerializdUsername").is_none());
}

#[test]
fn credentials_update_the_matching_account_in_place() {
    let mut config = config();

    assert!(!apply_credentials(
        &mut config,
        BOB,
        " BOB@example.com ",
        "rotated",
        None
    ));

    let account = &accounts(&config)[0];
    assert_eq!(accounts(&config).len(), 1);
    assert_eq!(account["Password"], json!("rotated"));
    assert_eq!(account["Email"], json!("BOB@example.com"));
    assert_eq!(account["SerializdUsername"], json!("bobwatches"));
}

#[test]
fn an_updated_account_keeps_its_own_toggles() {
    let mut config = config();

    apply_credentials(&mut config, BOB, "bob@example.com", "rotated", None);

    let account = &accounts(&config)[0];
    assert_eq!(account["WatchlistName"], json!("Bob's shows"));
    assert_eq!(account["AFieldFromANewerPluginVersion"], json!(7));
    assert_eq!(account["SyncWatchlist"], json!(true));
}

#[test]
fn credentials_for_a_new_email_add_a_second_account() {
    let mut config = config();

    assert!(apply_credentials(
        &mut config,
        BOB,
        "robert@example.com",
        "other",
        None
    ));

    assert_eq!(accounts(&config).len(), 2);
    assert_eq!(accounts(&config)[0]["Password"], json!("hunter2"));
}

#[test]
fn a_blank_email_is_filled_in_place() {
    let mut config = json!({
        "SerializdAccounts": [{ "UserJellyfinId": ALICE, "Email": "" }]
    });

    assert!(!apply_credentials(
        &mut config,
        ALICE,
        "alice@example.com",
        "s3cret",
        None
    ));

    assert_eq!(accounts(&config).len(), 1);
    assert_eq!(accounts(&config)[0]["Email"], json!("alice@example.com"));
}

#[test]
fn credentials_leave_other_users_alone() {
    let mut config = config();
    let before = accounts(&config)[0].clone();

    apply_credentials(&mut config, ALICE, "alice@example.com", "s3cret", None);

    assert_eq!(accounts(&config)[0], before);
}

#[test]
fn the_letterboxd_side_is_untouched() {
    let mut config = config();
    let letterboxd = config["Accounts"].clone();

    apply_credentials(&mut config, ALICE, "alice@example.com", "s3cret", None);

    assert_eq!(config["Accounts"], letterboxd);
    assert_eq!(config["JellyseerrUrl"], json!("https://seer.example"));
}

#[test]
fn an_absent_serializd_list_becomes_an_array() {
    let mut config = json!({ "Accounts": [] });

    assert!(apply_credentials(
        &mut config,
        ALICE,
        "alice@example.com",
        "s3cret",
        None
    ));
    assert_eq!(accounts(&config).len(), 1);
}

#[test]
fn a_created_account_carries_the_id_form_the_plugin_matches_on() {
    let dashed = "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa";
    let mut config = json!({ "SerializdAccounts": [] });

    apply_credentials(&mut config, dashed, "alice@example.com", "s3cret", None);

    assert_eq!(accounts(&config)[0]["UserJellyfinId"], json!(ALICE));
}

#[test]
fn find_reads_only_the_users_own_account() {
    let config = config();

    assert_eq!(
        find(&config, BOB),
        Some(Account { username: Some("bobwatches".to_owned()), enabled: true })
    );
    assert_eq!(find(&config, ALICE), None);
}

#[test]
fn find_prefers_an_account_with_an_email() {
    let config = json!({
        "SerializdAccounts": [
            { "UserJellyfinId": ALICE, "Email": "", "Enabled": true },
            { "UserJellyfinId": ALICE, "Email": "alice@example.com", "Enabled": false }
        ]
    });

    assert_eq!(
        find(&config, ALICE),
        Some(Account { username: None, enabled: false })
    );
}
