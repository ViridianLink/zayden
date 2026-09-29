#![cfg(feature = "ssr")]
//! The editor speaks strings; the destiny2 crate speaks enums. Converting a
//! stored loadout to the form and back must be lossless, or opening and saving a
//! loadout unchanged would silently rewrite it, and an unknown option string must
//! name the field instead of being coerced.

use dashboard::dto::destiny2::{ArmourForm, LoadoutForm, StatForm};
use dashboard::server::destiny2::{editor_form, loadout_form, raw_loadout};
use destiny2::endgame_analysis::sheet::Affinity;
use destiny2::loadouts::{
    Archetype,
    ArmourSlot,
    Class,
    Element,
    Mode,
    RawArmour,
    RawAspect,
    RawLoadout,
    RawWeapon,
    StatKind,
};

fn raw() -> RawLoadout {
    RawLoadout {
        name: "Form Trip".into(),
        class: Class::Hunter,
        element: Element::Void,
        mode: Mode::PvP,
        tags: vec!["Trials".into()],
        super_name: "Shadowshot".into(),
        super_emoji: "shadowshot".into(),
        class_ability: "gamblers_dodge".into(),
        jump: "triple_jump".into(),
        melee: "snare_bomb".into(),
        grenade: "vortex_grenade".into(),
        aspects: vec![RawAspect {
            aspect: "stylish_executioner".into(),
            fragments: vec!["echo_of_starvation".into()],
        }],
        weapons: vec![RawWeapon {
            name: "Test".into(),
            affinity: Affinity::Kinetic,
            archetype: Archetype::HandCannon,
            icon_url: "https://www.bungie.net/t.jpg".into(),
            perks: vec!["opening_shot".into()],
        }],
        armour: vec![RawArmour {
            slot: ArmourSlot::ClassItem,
            name: "Cloak".into(),
            icon_url: "https://www.bungie.net/k.jpg".into(),
            mods: vec![],
        }],
        stats: vec![(StatKind::Weapons, 100)],
        artifact_name: String::new(),
        artifact_perks: vec![],
        author: "Oscar".into(),
        dim_link: "https://dim.gg/y".into(),
        video_url: String::new(),
        how_it_works: String::new(),
    }
}

#[test]
fn a_stored_loadout_survives_the_form_round_trip() {
    let form = loadout_form(Some(7), raw());
    assert_eq!(form.id, Some(7));
    assert_eq!(raw_loadout(&form).unwrap(), raw());
}

#[test]
fn an_unknown_option_names_its_field() {
    let mut form = loadout_form(None, raw());
    form.class = "Gunslinger".into();
    assert_eq!(
        raw_loadout(&form).unwrap_err().to_string(),
        "unknown class `Gunslinger`"
    );
}

#[test]
fn a_non_numeric_stat_value_is_rejected() {
    let mut form: LoadoutForm = loadout_form(None, raw());
    form.stats = vec![StatForm { stat: "weapons".into(), value: "lots".into() }];
    assert!(raw_loadout(&form).is_err());
}

#[test]
fn blank_stat_rows_are_ignored() {
    let mut form = loadout_form(None, raw());
    form.stats.push(StatForm { stat: String::new(), value: String::new() });
    form.stats.push(StatForm { stat: "health".into(), value: "  ".into() });
    assert_eq!(raw_loadout(&form).unwrap(), raw());
}

#[test]
fn the_editor_offers_every_missing_armour_slot_and_stat() {
    let form = editor_form(Some(7), raw());
    assert_eq!(form.id, Some(7));

    let slots: Vec<String> =
        ArmourSlot::ALL.iter().map(ToString::to_string).collect();
    let form_slots: Vec<String> =
        form.armour.iter().map(|a| a.slot.clone()).collect();
    assert_eq!(form_slots, slots);
    assert_eq!(form.armour[4].name, "Cloak");
    assert_eq!(form.armour[0], ArmourForm {
        slot: "Helmet".into(),
        ..ArmourForm::default()
    });

    assert_eq!(form.stats[0], StatForm {
        stat: "weapons".into(),
        value: "100".into()
    });
    let missing: Vec<StatForm> = StatKind::ALL
        .iter()
        .filter(|s| **s != StatKind::Weapons)
        .map(|s| StatForm { stat: s.to_string(), value: String::new() })
        .collect();
    assert_eq!(form.stats[1..], missing[..]);
}

#[test]
fn the_editor_overlay_saves_back_unchanged() {
    assert_eq!(raw_loadout(&editor_form(Some(7), raw())).unwrap(), raw());
}

#[test]
fn a_loadout_without_armour_or_stats_parses_from_its_blank_rows() {
    let mut bare = raw();
    bare.armour = vec![];
    bare.stats = vec![];

    let form = editor_form(None, bare.clone());
    assert_eq!(form.armour.len(), ArmourSlot::ALL.len());
    assert_eq!(form.stats.len(), StatKind::ALL.len());
    assert_eq!(raw_loadout(&form).unwrap(), bare);
}

#[test]
fn unknown_weapon_and_armour_options_name_their_field() {
    let mut form = loadout_form(None, raw());
    form.weapons[0].affinity = "Radiant".into();
    assert_eq!(
        raw_loadout(&form).unwrap_err().to_string(),
        "unknown affinity `Radiant`"
    );

    let mut form = loadout_form(None, raw());
    form.weapons[0].archetype = "Spoon".into();
    assert_eq!(
        raw_loadout(&form).unwrap_err().to_string(),
        "unknown archetype `Spoon`"
    );

    let mut form = loadout_form(None, raw());
    form.armour[0].slot = "Cape".into();
    assert_eq!(
        raw_loadout(&form).unwrap_err().to_string(),
        "unknown armour slot `Cape`"
    );
}
