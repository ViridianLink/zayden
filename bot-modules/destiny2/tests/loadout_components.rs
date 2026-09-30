//! Discord refuses a message with more than 40 components, nested ones and the
//! container included, so `/destiny2 builds` would fail for that loadout. The
//! editor meter trusts `budget::components`; it must match what the builder
//! emits, and every build the editor accepts must fit.

use destiny2::endgame_analysis::sheet::Affinity;
use destiny2::loadouts::{
    Archetype,
    ArmourRecord,
    ArmourSlot,
    AspectRecord,
    Class,
    Element,
    LoadoutRecord,
    Mode,
    StatKind,
    WeaponRecord,
    budget,
    limits,
};
use serde_json::Value;
use serenity::all::EmojiId;
use zayden_core::EmojiCache;

fn keys(prefix: &str, n: usize) -> Vec<String> {
    (0..n).map(|i| format!("{prefix}_{i}")).collect()
}

fn record(tags: usize, weapons: usize, armour: usize, misc: bool) -> LoadoutRecord {
    LoadoutRecord {
        id: 1,
        name: "Full".into(),
        class: Class::Titan,
        element: Element::Arc,
        mode: Mode::PvE,
        tags: keys("tag", tags),
        super_name: "Thundercrash".into(),
        super_emoji: "thundercrash".into(),
        class_ability: "rally_barricade".into(),
        jump: "catapult_lift".into(),
        melee: "thunderclap".into(),
        grenade: "pulse_grenade".into(),
        aspects: (0..limits::ASPECTS)
            .map(|i| AspectRecord {
                emoji: format!("aspect_{i}"),
                fragments: keys(&format!("fragment_{i}"), 3),
            })
            .collect(),
        weapons: (0..weapons)
            .map(|i| WeaponRecord {
                name: format!("Weapon {i}"),
                affinity: Affinity::Arc,
                archetype: Archetype::AutoRifle,
                icon_url: "https://www.bungie.net/w.jpg".into(),
                perks: keys("perk", limits::PERKS),
            })
            .collect(),
        armour: ArmourSlot::ALL
            .into_iter()
            .take(armour)
            .map(|slot| ArmourRecord {
                slot,
                name: format!("{slot} piece"),
                icon_url: "https://www.bungie.net/a.jpg".into(),
                mods: keys("mod", limits::MODS),
            })
            .collect(),
        stats: if misc { vec![(StatKind::Grenade, 200)] } else { vec![] },
        artifact_name: None,
        artifact_perks: if misc { keys("artifact", 3) } else { vec![] },
        author: "Oscar".into(),
        dim_link: "https://dim.gg/x".into(),
        video_url: None,
        how_it_works: misc.then(|| "Throw grenades.".into()),
    }
}

fn cache() -> EmojiCache {
    let mut names: Vec<String> = [
        "arc",
        "thundercrash",
        "rally_barricade",
        "catapult_lift",
        "thunderclap",
        "pulse_grenade",
        "grenade",
    ]
    .map(str::to_owned)
    .into();
    names.extend(keys("aspect", limits::ASPECTS));
    for i in 0..limits::ASPECTS {
        names.extend(keys(&format!("fragment_{i}"), 3));
    }
    names.extend(keys("perk", limits::PERKS));
    names.extend(keys("mod", limits::MODS));
    names.extend(keys("artifact", 3));
    names.into_iter().zip((1..).map(EmojiId::new)).collect()
}

/// Every JSON object with a `type` is one Discord component.
fn count(v: &Value) -> usize {
    match v {
        Value::Object(map) => {
            usize::from(map.get("type").is_some_and(Value::is_u64))
                + map.values().map(count).sum::<usize>()
        },
        Value::Array(items) => items.iter().map(count).sum(),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 0,
    }
}

// A macro so `unwrap` expands inside the `#[test]` that clippy exempts.
macro_rules! emitted {
    ($record:expr) => {{
        let component = $record.container(&cache()).unwrap();
        count(&serde_json::to_value(component).unwrap())
    }};
}

#[test]
fn the_formula_matches_the_built_message() {
    for tags in 0..=limits::TAGS {
        for weapons in 0..=limits::WEAPONS {
            for armour in 0..=ArmourSlot::ALL.len() {
                for misc in [false, true] {
                    let r = record(tags, weapons, armour, misc);
                    assert_eq!(
                        emitted!(r),
                        budget::components(tags, weapons, armour, misc),
                        "tags {tags}, weapons {weapons}, armour {armour}, misc {misc}"
                    );
                }
            }
        }
    }
}

#[test]
fn every_build_the_editor_allows_fits_discords_limit() {
    let largest = record(limits::TAGS, limits::WEAPONS, ArmourSlot::ALL.len(), true);
    assert_eq!(emitted!(largest), budget::MAX_COMPONENTS);
}

#[test]
fn a_full_build_without_tags_keeps_its_spacer() {
    let base =
        budget::base_components(0, limits::WEAPONS, ArmourSlot::ALL.len(), true);
    assert!(budget::has_spacer(limits::WEAPONS, base));
    assert_eq!(
        budget::components(0, limits::WEAPONS, ArmourSlot::ALL.len(), true),
        38
    );
}

#[test]
fn the_spacer_is_dropped_only_when_it_would_not_fit() {
    let base = budget::base_components(
        limits::TAGS,
        limits::WEAPONS,
        ArmourSlot::ALL.len(),
        true,
    );
    assert_eq!(base, budget::MAX_COMPONENTS);
    assert!(!budget::has_spacer(limits::WEAPONS, base));
    assert!(!budget::has_spacer(0, 20), "no weapons, nothing to separate");
}
