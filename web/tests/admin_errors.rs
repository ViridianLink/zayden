//! Admin errors render as the bare message the dashboard shows, and only
//! refusals count as denials.

use destiny2::SaveError;
use web::admin::{
    AdminError,
    EmojiUploadError,
    LoadoutFieldError,
    LoadoutFormError,
    blank,
    loadout_check,
};
use web::auth::AuthError;
use web::util::server_error_text;

#[test]
fn refusals_are_denials_and_failures_are_not() {
    assert!(AdminError::from(AuthError::Unauthenticated).is_denied());
    assert!(AdminError::from(AuthError::Forbidden).is_denied());
    assert!(!AdminError::from(AuthError::Database("down".to_owned())).is_denied());
    assert!(!AdminError::LoadoutNotFound(5).is_denied());
    assert!(!AdminError::Discord("401".to_owned()).is_denied());
}

#[test]
fn loadout_errors_keep_the_server_text() {
    assert_eq!(
        AdminError::LoadoutNotFound(7).to_string(),
        "loadout 7 does not exist"
    );
    assert_eq!(
        server_error_text(AdminError::LoadoutNotFound(7)),
        "error running server function: loadout 7 does not exist"
    );
    assert_eq!(
        AdminError::from(SaveError::DuplicateName).to_string(),
        "a loadout with this class, element and name already exists"
    );
    assert_eq!(
        AdminError::from(SaveError::NotFound(3)).to_string(),
        "loadout 3 does not exist"
    );
    assert_eq!(
        AdminError::ZaydenIdNotConfigured.to_string(),
        "zayden_id is not configured"
    );
    assert_eq!(AdminError::from(AuthError::Forbidden).to_string(), "forbidden");
}

#[test]
fn form_errors_name_the_field_or_value() {
    assert_eq!(
        LoadoutFormError::UnknownOption { field: "mode", value: "Raid".to_owned() }
            .to_string(),
        "unknown mode `Raid`"
    );
    assert_eq!(
        LoadoutFormError::StatValue("lots".to_owned()).to_string(),
        "stat value `lots` is not a whole number"
    );
    assert_eq!(
        LoadoutFieldError::UnknownField("colour".to_owned()).to_string(),
        "unknown field `colour`"
    );
    assert_eq!(
        LoadoutFieldError::RepeatedField("name".to_owned()).to_string(),
        "field `name` appears more than once"
    );
    assert_eq!(
        LoadoutFieldError::InvalidId("x".to_owned()).to_string(),
        "loadout id `x` is not a whole number"
    );
}

#[test]
fn emoji_upload_errors_match_the_create_panel_text() {
    let cases = [
        (
            EmojiUploadError::InvalidName,
            "emoji names are 2-32 lowercase letters, digits or underscores",
        ),
        (
            EmojiUploadError::ReservedName("arc".to_owned()),
            "`arc` is reserved for a built-in class, element, weapon or stat icon",
        ),
        (
            EmojiUploadError::NameTaken("knockout".to_owned()),
            "Zayden already has an emoji named `knockout`",
        ),
        (EmojiUploadError::NotHttps, "the image link must be an https:// URL"),
        (
            EmojiUploadError::PrivateAddress,
            "that link points at a private network address",
        ),
        (
            EmojiUploadError::Fetch("timed out".to_owned()),
            "couldn't download the image: timed out",
        ),
        (EmojiUploadError::TooLarge, "images must be 256 KiB or smaller"),
        (EmojiUploadError::UnsupportedType, "images must be PNG, JPEG, GIF or WebP"),
        (EmojiUploadError::BadDataUri, "the chosen file couldn't be read"),
        (
            EmojiUploadError::Discord("400".to_owned()),
            "Discord refused the emoji: 400",
        ),
    ];
    for (error, text) in cases {
        assert_eq!(AdminError::from(error).to_string(), text);
    }
}

/// The emoji-create panel and the budget meter show their message as is,
/// without the prefix load, save and delete failures carry.
#[test]
fn emoji_and_budget_errors_are_bare() {
    let emoji = AdminError::from(EmojiUploadError::InvalidName).to_string();
    assert_eq!(
        emoji,
        "emoji names are 2-32 lowercase letters, digits or underscores"
    );
    assert!(!emoji.starts_with("error running server function"));

    let budget = loadout_check(&blank()).error;
    assert_eq!(budget.as_deref(), Some("name is required"));
}
