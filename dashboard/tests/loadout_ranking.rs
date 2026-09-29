#![cfg(feature = "ssr")]
//! Emoji keys have no metadata, so a picker ranks them by how existing builds
//! use them. Enum icons (`solar`, `titan`, `auto_rifle`) illustrate the enum
//! selectors and must never be offered as an ability, mod or perk.

use dashboard::dto::destiny2::{
    ArmourPieceInfo,
    CatalogWeaponInfo,
    EmojiInfo,
    LoadoutCatalog,
    LoadoutForm,
    LoadoutOptions,
    UsageInfo,
};
use dashboard::dto::destiny2_keys::{
    display_name,
    enum_key,
    is_valid_key,
    key_from_query,
};
use dashboard::ui::pages::destiny2_loadouts::ranking::{
    CatalogIndex,
    Field,
    Scope,
    Section,
    matches,
    sections,
};

fn usage(
    field: &str,
    key: &str,
    class: &str,
    element: &str,
    uses: u32,
) -> UsageInfo {
    UsageInfo {
        field: field.into(),
        key: key.into(),
        class: class.into(),
        element: element.into(),
        uses,
    }
}

fn emoji(name: &str) -> EmojiInfo {
    EmojiInfo { name: name.into(), id: format!("{}", name.len()) }
}

fn catalog() -> LoadoutCatalog {
    LoadoutCatalog {
        weapons: vec![
            CatalogWeaponInfo {
                name: "Ace of Spades".into(),
                affinity: "Kinetic".into(),
                archetype: "Hand Cannon".into(),
                icon_url: "https://www.bungie.net/ace.jpg".into(),
                known_perks: vec!["memento_mori".into()],
            },
            CatalogWeaponInfo {
                name: "Adamantite".into(),
                affinity: "Arc".into(),
                archetype: "Auto Rifle".into(),
                icon_url: String::new(),
                known_perks: vec![],
            },
        ],
        perks: vec!["memento_mori".into(), "field_prep".into()],
        emojis: [
            "spark_of_shock",
            "spark_of_ions",
            "ember_of_ashes",
            "spark_of_beacons",
            "arc",
            "titan",
            "auto_rifle",
        ]
        .into_iter()
        .map(emoji)
        .collect(),
        usage: vec![
            usage("fragment", "spark_of_ions", "Titan", "Arc", 1),
            usage("fragment", "spark_of_shock", "Titan", "Arc", 3),
            usage("fragment", "ember_of_ashes", "Warlock", "Solar", 2),
            usage("fragment", "spark_of_shock", "Hunter", "Arc", 1),
            usage("weapon", "Adamantite", "Titan", "Arc", 1),
            usage("artifact", "Tablet of Ruin", "Titan", "Arc", 2),
            usage("artifact", "Nether Codex", "Warlock", "Void", 1),
        ],
        super_names: vec![],
        armour: vec![
            ArmourPieceInfo {
                slot: "Helmet".into(),
                class: "Titan".into(),
                name: "Helm of Saint-14".into(),
                icon_url: String::new(),
            },
            ArmourPieceInfo {
                slot: "Helmet".into(),
                class: "Warlock".into(),
                name: "Nezarec's Sin".into(),
                icon_url: String::new(),
            },
            ArmourPieceInfo {
                slot: "Chest".into(),
                class: "Titan".into(),
                name: "Synthoceps".into(),
                icon_url: String::new(),
            },
        ],
        options: LoadoutOptions {
            classes: vec!["Hunter".into(), "Titan".into(), "Warlock".into()],
            elements: vec!["Arc".into(), "Solar".into()],
            archetypes: vec!["Auto Rifle".into()],
            ..LoadoutOptions::default()
        },
        blank: LoadoutForm::default(),
    }
}

fn run(
    field: Field,
    taken: &[String],
    weapon: Option<&str>,
    slot: Option<&str>,
    query: &str,
) -> Vec<Section> {
    let c = catalog();
    let idx = CatalogIndex::new(&c);
    let scope = Scope {
        field,
        class: "Titan",
        element: "Arc",
        weapon,
        armour_slot: slot,
        taken,
    };
    sections(&c, &idx, &scope, query)
}

fn keys(s: &Section) -> Vec<&str> {
    s.items.iter().map(|c| c.key.as_str()).collect()
}

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
fn every_query_word_must_match() {
    assert!(matches("ember ash", "Ember of Ashes", "ember_of_ashes"));
    assert!(matches("", "Anything", "anything"));
    assert!(!matches("ember ion", "Ember of Ashes", "ember_of_ashes"));
}

#[test]
fn same_build_usage_ranks_first_then_elsewhere_then_other_emojis() {
    let s = run(Field::Fragment, &[], None, None, "");
    let titles: Vec<&str> = s.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(titles, [
        "Used in Arc Titan builds",
        "Used elsewhere",
        "Other Zayden emojis"
    ]);
    assert_eq!(keys(&s[0]), ["spark_of_shock", "spark_of_ions"]);
    assert_eq!(keys(&s[1]), ["ember_of_ashes"]);
    assert_eq!(keys(&s[2]), ["spark_of_beacons"]);
}

#[test]
fn enum_icons_are_never_offered_and_keys_appear_once() {
    let s = run(Field::Fragment, &[], None, None, "");
    let all: Vec<&str> = s.iter().flat_map(keys).collect();
    for reserved in ["arc", "titan", "auto_rifle"] {
        assert!(!all.contains(&reserved), "{reserved}");
    }
    let mut unique = all.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), all.len());
}

#[test]
fn taken_keys_are_marked_selected() {
    let taken = vec!["spark_of_shock".to_owned()];
    let s = run(Field::Fragment, &taken, None, None, "");
    let shock = s[0].items.iter().find(|c| c.key == "spark_of_shock").unwrap();
    assert!(shock.selected);
    assert!(!s[0].items.iter().find(|c| c.key == "spark_of_ions").unwrap().selected);
}

#[test]
fn a_query_filters_and_drops_empty_sections() {
    let s = run(Field::Fragment, &[], None, None, "ember");
    assert_eq!(s.len(), 1);
    assert_eq!(keys(&s[0]), ["ember_of_ashes"]);
}

#[test]
fn weapon_perks_open_with_the_weapons_known_perks() {
    let s = run(Field::WeaponPerk, &[], Some("Ace of Spades"), None, "");
    assert_eq!(s[0].title, "Known perks for Ace of Spades");
    assert_eq!(keys(&s[0]), ["memento_mori"]);
    assert!(s.iter().flat_map(keys).any(|k| k == "field_prep"));
}

#[test]
fn weapons_rank_this_builds_weapons_first() {
    let s = run(Field::Weapon, &[], None, None, "");
    assert_eq!(keys(&s[0]), ["Adamantite"]);
    assert_eq!(keys(&s[1]), ["Ace of Spades"]);
    assert_eq!(s[1].items[0].detail, "Kinetic Hand Cannon");
    assert_eq!(s[0].items[0].image, None);
}

#[test]
fn armour_offers_only_the_requested_slot_own_class_first() {
    let s = run(Field::Armour, &[], None, Some("Helmet"), "");
    assert_eq!(keys(&s[0]), ["Helm of Saint-14"]);
    assert_eq!(s[1].title, "Other classes");
    assert_eq!(keys(&s[1]), ["Nezarec's Sin"]);
}

#[test]
fn emoji_images_resolve_through_the_index() {
    let c = catalog();
    let idx = CatalogIndex::new(&c);
    assert_eq!(
        idx.image("arc").as_deref(),
        Some("https://cdn.discordapp.com/emojis/3.webp?size=64"),
    );
    assert_eq!(idx.enum_image("Auto Rifle"), idx.image("auto_rifle"));
    assert!(idx.is_reserved("titan"));
    assert!(!idx.has_emoji("nope"));
}

#[test]
fn artifacts_are_named_as_typed_and_ranked_by_build() {
    let s = run(Field::Artifact, &[], None, None, "");
    assert_eq!(keys(&s[0]), ["Tablet of Ruin"]);
    assert_eq!(keys(&s[1]), ["Nether Codex"]);
    assert_eq!(s.len(), 2, "artifacts never list emojis");
    assert_eq!(s[1].items[0].label, "Nether Codex");
}
