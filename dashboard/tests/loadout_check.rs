#![cfg(feature = "ssr")]
//! The editor's budget meter and live error come from `loadout_check`, which
//! must agree with what `save_loadout` accepts, or the meter would promise a
//! save that Discord's limits then refuse.

use dashboard::dto::destiny2::{ArmourForm, LoadoutForm, WeaponForm};
use dashboard::server::destiny2::{loadout_check, loadout_form};
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
};

fn form() -> LoadoutForm {
    loadout_form(None, RawLoadout {
        name: "Check".into(),
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
            name: "W".into(),
            affinity: Affinity::Arc,
            archetype: Archetype::AutoRifle,
            icon_url: "https://www.bungie.net/w.jpg".into(),
            perks: vec!["field_prep".into()],
        }],
        armour: vec![RawArmour {
            slot: ArmourSlot::Helmet,
            name: "Helm".into(),
            icon_url: "https://www.bungie.net/h.jpg".into(),
            mods: vec![],
        }],
        stats: vec![],
        artifact_name: String::new(),
        artifact_perks: vec![],
        author: "Oscar".into(),
        dim_link: "https://dim.gg/x".into(),
        video_url: String::new(),
        how_it_works: String::new(),
    })
}

#[test]
fn a_valid_form_reports_its_budget_and_no_error() {
    let check = loadout_check(&form());
    assert_eq!(check.error, None);
    assert!(check.text.is_some_and(|t| t > 0 && t <= check.max_text));
    assert_eq!(check.components, 12 + 1 + 3 * (1 + 1) + 1);
    assert_eq!(check.max_components, 40);
}

#[test]
fn blank_armour_rows_do_not_count_towards_components() {
    let mut f = form();
    f.armour.push(ArmourForm { slot: "Chest".into(), ..ArmourForm::default() });
    assert_eq!(loadout_check(&f).components, loadout_check(&form()).components);
}

#[test]
fn the_largest_build_fits_discords_component_limit() {
    let mut f = form();
    f.tags = vec!["a".into(), "b".into(), "c".into()];
    f.weapons = vec![f.weapons[0].clone(); 3];
    f.weapons.iter_mut().zip(["A", "B", "C"]).for_each(|(w, n)| {
        *w = WeaponForm { name: n.into(), ..w.clone() };
    });
    f.armour = ["Helmet", "Arms", "Chest", "Legs", "Class Item"]
        .into_iter()
        .map(|slot| ArmourForm {
            slot: slot.into(),
            name: format!("{slot} piece"),
            icon_url: "https://www.bungie.net/a.jpg".into(),
            mods: vec![],
        })
        .collect();
    f.how_it_works = "Throw grenades.".into();

    let check = loadout_check(&f);
    assert_eq!(check.components, check.max_components);
    assert_eq!(check.error, None);
    assert!(check.text.is_some());
}

#[test]
fn an_unknown_option_names_the_field() {
    let mut f = form();
    f.class = "Gardener".into();
    let check = loadout_check(&f);
    assert!(check.error.is_some_and(|e| e.contains("unknown class")));
    assert_eq!(check.text, None);
}
