//! Parsing a raw loadout is the only way to obtain a `LoadoutDraft`, and the DB
//! writer accepts nothing else, so these limits are what stand between the
//! website form and a `/destiny2 builds` render that Discord rejects: action rows
//! hold five buttons, button ids must be unique, and emoji names follow Discord's
//! 2-32 `[a-z0-9_]` rule.

use destiny2::DraftError;
use destiny2::endgame_analysis::sheet::Affinity;
use destiny2::loadouts::{
    Archetype,
    ArmourSlot,
    Class,
    Element,
    EmojiKey,
    LoadoutDraft,
    Mode,
    RawArmour,
    RawAspect,
    RawLoadout,
    RawWeapon,
    StatKind,
    limits,
};

fn valid() -> RawLoadout {
    RawLoadout {
        name: "Crest of Alpha Lupi".into(),
        class: Class::Titan,
        element: Element::Arc,
        mode: Mode::PvE,
        tags: vec!["Raid".into()],
        super_name: "Thundercrash".into(),
        super_emoji: "thundercrash".into(),
        class_ability: "rally_barricade".into(),
        jump: "catapult_lift".into(),
        melee: "thunderclap".into(),
        grenade: "pulse_grenade".into(),
        aspects: vec![RawAspect {
            aspect: "knockout".into(),
            fragments: vec!["spark_of_shock".into()],
        }],
        weapons: vec![RawWeapon {
            name: "Mint Retrograde".into(),
            affinity: Affinity::Strand,
            archetype: Archetype::RocketPulseRifle,
            icon_url: "https://www.bungie.net/a.jpg".into(),
            perks: vec!["field_prep".into()],
        }],
        armour: vec![RawArmour {
            slot: ArmourSlot::Helmet,
            name: "War Numen's Crown".into(),
            icon_url: "https://www.bungie.net/b.jpg".into(),
            mods: vec!["void_siphon".into()],
        }],
        stats: vec![(StatKind::Weapons, 200), (StatKind::Super, 150)],
        artifact_name: String::new(),
        artifact_perks: vec![],
        author: "LlamaD2".into(),
        dim_link: "https://dim.gg/abc".into(),
        video_url: String::new(),
        how_it_works: String::new(),
    }
}

// A macro so `unwrap_err` expands inside the `#[test]` that clippy exempts.
macro_rules! err {
    ($raw:expr) => {
        LoadoutDraft::try_from($raw).unwrap_err()
    };
}

#[test]
fn a_complete_loadout_parses() {
    assert!(LoadoutDraft::try_from(valid()).is_ok());
}

#[test]
fn emoji_keys_follow_discords_name_rule() {
    assert!("spark_of_shock".parse::<EmojiKey>().is_ok());
    assert!("a".parse::<EmojiKey>().is_err());
    assert!("Spark".parse::<EmojiKey>().is_err());
    assert!("spark-of-shock".parse::<EmojiKey>().is_err());
    assert!("a".repeat(33).parse::<EmojiKey>().is_err());
}

#[test]
fn blank_required_fields_are_named() {
    let mut raw = valid();
    raw.name = "   ".into();
    assert_eq!(err!(raw), DraftError::Required { field: "name" });

    let mut raw = valid();
    raw.melee = String::new();
    assert_eq!(err!(raw), DraftError::Required { field: "melee" });
}

#[test]
fn too_many_tags_would_overflow_the_action_row() {
    let mut raw = valid();
    raw.tags = vec!["a".into(), "b".into(), "c".into(), "d".into()];
    assert_eq!(err!(raw), DraftError::TooMany { field: "tags", max: limits::TAGS });
}

#[test]
fn a_tag_cannot_reuse_another_buttons_id() {
    let mut raw = valid();
    raw.tags = vec!["arc".into()];
    assert_eq!(err!(raw), DraftError::DuplicateTag("arc".into()));

    let mut raw = valid();
    raw.tags = vec!["PvE".into()];
    assert_eq!(err!(raw), DraftError::DuplicateTag("PvE".into()));
}

#[test]
fn blank_weapon_rows_are_dropped_and_a_weaponless_build_parses() {
    let mut raw = valid();
    raw.weapons[0].name = String::new();
    raw.weapons[0].icon_url = String::new();
    assert!(LoadoutDraft::try_from(raw).is_ok());
}

#[test]
fn list_caps_are_enforced() {
    let mut raw = valid();
    raw.aspects = vec![raw.aspects[0].clone(); 3];
    assert_eq!(err!(raw), DraftError::TooMany {
        field: "aspects",
        max: limits::ASPECTS
    });

    let mut raw = valid();
    raw.weapons = vec![raw.weapons[0].clone(); 4];
    assert_eq!(err!(raw), DraftError::TooMany {
        field: "weapons",
        max: limits::WEAPONS
    });
}

#[test]
fn armour_slots_and_stats_are_unique() {
    let mut raw = valid();
    raw.armour.push(raw.armour[0].clone());
    assert_eq!(err!(raw), DraftError::DuplicateArmourSlot(ArmourSlot::Helmet));

    let mut raw = valid();
    raw.stats.push((StatKind::Weapons, 10));
    assert_eq!(err!(raw), DraftError::DuplicateStat(StatKind::Weapons));
}

#[test]
fn stat_values_are_bounded() {
    let mut raw = valid();
    raw.stats = vec![(StatKind::Health, 201)];
    assert_eq!(err!(raw), DraftError::StatOutOfRange {
        stat: StatKind::Health,
        value: 201,
        max: limits::STAT_MAX
    });
}

#[test]
fn links_must_be_https() {
    let mut raw = valid();
    raw.dim_link = "http://dim.gg/abc".into();
    assert_eq!(err!(raw), DraftError::NotHttps { field: "DIM link" });

    let mut raw = valid();
    raw.weapons[0].icon_url = "javascript:alert(1)".into();
    assert_eq!(err!(raw), DraftError::NotHttps { field: "weapon icon" });
}

#[test]
fn over_long_text_is_rejected() {
    let mut raw = valid();
    raw.how_it_works = "x".repeat(limits::TEXT + 1);
    assert_eq!(err!(raw), DraftError::TooLong {
        field: "how it works",
        max: limits::TEXT
    });
}

fn armour(slot: ArmourSlot) -> RawArmour {
    RawArmour {
        slot,
        name: format!("{slot} Piece"),
        icon_url: "https://www.bungie.net/b.jpg".into(),
        mods: vec![],
    }
}

#[test]
fn the_largest_build_discord_can_render_parses() {
    let mut raw = valid();
    raw.weapons = vec![raw.weapons[0].clone(); limits::WEAPONS];
    raw.armour = ArmourSlot::ALL[..4].iter().copied().map(armour).collect();
    raw.tags = vec!["Raid".into(), "GM".into()];
    assert!(LoadoutDraft::try_from(raw).is_ok());
}

#[test]
fn a_build_over_discords_component_limit_is_rejected() {
    let mut raw = valid();
    raw.weapons = vec![raw.weapons[0].clone(); limits::WEAPONS];
    raw.armour = ArmourSlot::ALL.iter().copied().map(armour).collect();
    raw.tags = vec![];
    let e = err!(raw);
    assert_eq!(e, DraftError::TooManyComponents { needed: 41, max: 40 });
    assert_eq!(
        e.to_string(),
        "this build needs 41 discord components, the limit is 40; remove a \
         weapon, armour piece or tag"
    );
}

#[test]
fn a_build_over_discords_text_limit_is_rejected() {
    let long_key = "k".repeat(32);
    let mut raw = valid();
    raw.how_it_works = "x".repeat(limits::TEXT);
    raw.artifact_perks = vec![long_key.clone(); limits::ARTIFACT_PERKS];
    raw.aspects = vec![
        RawAspect {
            aspect: long_key.clone(),
            fragments: vec![long_key.clone(); limits::FRAGMENTS],
        };
        limits::ASPECTS
    ];
    raw.weapons[0].perks = vec![long_key; limits::PERKS];
    raw.weapons = vec![raw.weapons[0].clone(); limits::WEAPONS];
    assert!(matches!(
        err!(raw),
        DraftError::TooMuchText { estimate, max: 4000 } if estimate > 4000
    ));
}
