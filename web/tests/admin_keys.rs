//! Emoji keys are the shared vocabulary between the editor, the picker and
//! Discord: they must read back as names, come out of free-typed queries in a
//! form Discord accepts, and map enum labels to their icon emoji.

use web::admin::keys::{
    display_name,
    emoji_url,
    enum_key,
    is_valid_key,
    key_from_query,
};

#[test]
fn keys_read_as_names() {
    assert_eq!(display_name("spark_of_shock"), "Spark of Shock");
    assert_eq!(display_name("of_the_void"), "Of the Void");
    assert_eq!(display_name("x2_grenade"), "X2 Grenade");
}

#[test]
fn queries_become_valid_keys() {
    assert_eq!(key_from_query("Ember of Ashes!"), "ember_of_ashes");
    assert_eq!(
        key_from_query("  Hunter's  dodge - marksman "),
        "hunters_dodge_marksman"
    );
    assert!(key_from_query(&"a ".repeat(40)).len() <= 32);
    assert!(is_valid_key(&key_from_query("Spark of Shock")));
}

#[test]
fn key_validation_follows_discords_rule() {
    assert!(is_valid_key("ab"));
    assert!(is_valid_key(&"a".repeat(32)));
    assert!(!is_valid_key("a"));
    assert!(!is_valid_key(&"a".repeat(33)));
    assert!(!is_valid_key("Spark"));
    assert!(!is_valid_key("spark-of"));
}

#[test]
fn enum_labels_map_to_icon_keys() {
    assert_eq!(enum_key("Auto Rifle"), "auto_rifle");
    assert_eq!(enum_key("PvE"), "pve");
    assert_eq!(enum_key("Class Item"), "class_item");
}

#[test]
fn emoji_images_come_from_the_discord_cdn() {
    assert_eq!(emoji_url("3"), "https://cdn.discordapp.com/emojis/3.webp?size=64");
}
