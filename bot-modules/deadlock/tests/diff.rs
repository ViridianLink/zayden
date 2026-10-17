use deadlock::diff::{self, Change, PatchDiff, Presence, Snapshot};
use deadlock_api::model::{Hero, Item};
use serde_json::json;

macro_rules! hero {
    ($id:expr, $name:expr, $stats:expr, $slots:expr $(,)?) => {
        serde_json::from_value::<Hero>(json!({
            "id": $id,
            "class_name": format!("hero_{}", $id),
            "name": $name,
            "player_selectable": true,
            "starting_stats": $stats,
            "items": $slots
        }))
        .unwrap()
    };
}

macro_rules! unplayable {
    ($id:expr, $name:expr $(,)?) => {
        serde_json::from_value::<Hero>(json!({
            "id": $id,
            "class_name": format!("hero_{}", $id),
            "name": $name,
            "player_selectable": false
        }))
        .unwrap()
    };
}

macro_rules! ability {
    ($id:expr, $class_name:expr, $name:expr, $properties:expr, $upgrades:expr $(,)?) => {
        serde_json::from_value::<Item>(json!({
            "id": $id,
            "class_name": $class_name,
            "name": $name,
            "type": "ability",
            "properties": $properties,
            "upgrades": $upgrades
        }))
        .unwrap()
    };
}

macro_rules! weapon {
    ($class_name:expr, $clip_size:expr $(,)?) => {
        serde_json::from_value::<Item>(json!({
            "id": 900,
            "class_name": $class_name,
            "name": "Gun",
            "type": "weapon",
            "weapon_info": { "clip_size": $clip_size, "bullet_damage": 5.5 }
        }))
        .unwrap()
    };
}

macro_rules! shop_item {
    ($id:expr, $name:expr, $cost:expr $(,)?) => {
        serde_json::from_value::<Item>(json!({
            "id": $id,
            "class_name": format!("upgrade_{}", $id),
            "name": $name,
            "type": "upgrade",
            "shopable": true,
            "cost": $cost,
            "item_tier": 2
        }))
        .unwrap()
    };
}

fn between(
    old_heroes: &[Hero],
    old_items: &[Item],
    new_heroes: &[Hero],
    new_items: &[Item],
) -> PatchDiff {
    diff::diff(&Snapshot { heroes: old_heroes, items: old_items }, &Snapshot {
        heroes: new_heroes,
        items: new_items,
    })
}

fn change(label: &str, old: Option<&str>, new: Option<&str>) -> Change {
    Change {
        label: label.to_owned(),
        old: old.map(str::to_owned),
        new: new.map(str::to_owned),
    }
}

#[test]
fn a_changed_hero_stat_is_labelled_and_formatted() {
    let old = [hero!(1, "Alpha", json!({"max_health": {"value": 830}}), json!({}))];
    let new = [hero!(1, "Alpha", json!({"max_health": {"value": 850}}), json!({}))];

    let patch = between(&old, &[], &new, &[]);

    assert_eq!(patch.heroes.len(), 1);
    assert_eq!(patch.heroes[0].presence, Presence::Changed);
    assert_eq!(patch.heroes[0].stats, vec![change(
        "Max health",
        Some("830"),
        Some("850")
    )]);
}

#[test]
fn a_numeric_value_matching_its_text_form_is_not_a_change() {
    let old = [hero!(1, "Alpha", json!({"regen": {"value": 0.1}}), json!({}))];
    let new = [hero!(1, "Alpha", json!({"regen": {"value": "0.1"}}), json!({}))];

    assert!(between(&old, &[], &new, &[]).is_empty());
}

#[test]
fn a_weapon_stat_change_uses_its_fixed_label() {
    let slots = json!({"weapon_primary": "weapon_alpha"});
    let old = [hero!(1, "Alpha", json!({}), slots)];
    let new = [hero!(1, "Alpha", json!({}), slots)];

    let patch = between(&old, &[weapon!("weapon_alpha", 27)], &new, &[weapon!(
        "weapon_alpha",
        30,
    )]);

    assert_eq!(patch.heroes[0].weapon, vec![change(
        "Magazine size",
        Some("27"),
        Some("30")
    )]);
}

#[test]
fn ability_properties_append_a_postfix_only_to_plain_numbers() {
    let slots = json!({"signature1": "ability_dash"});
    let heroes = [hero!(1, "Alpha", json!({}), slots)];
    let properties = |cooldown: u32, speed: &str| {
        json!({
            "AbilityCooldown": {"value": cooldown, "label": "Cooldown", "postfix": "s"},
            "DashSpeed": {"value": speed, "label": "Dash speed", "postfix": "m"},
            "Hidden": {"value": cooldown}
        })
    };

    let patch = between(
        &heroes,
        &[ability!(10, "ability_dash", "Dash", properties(25, "12m"), json!([]))],
        &heroes,
        &[ability!(10, "ability_dash", "Dash", properties(22, "14m"), json!([]))],
    );

    let abilities = &patch.heroes[0].abilities;
    assert_eq!(abilities.len(), 1);
    assert_eq!(abilities[0].presence, Presence::Changed);
    assert_eq!(abilities[0].name, "Dash");
    assert_eq!(abilities[0].changes, vec![
        change("Cooldown", Some("25s"), Some("22s")),
        change("Dash speed", Some("12m"), Some("14m")),
    ]);
}

#[test]
fn upgrade_tiers_are_compared_in_order() {
    let heroes =
        [hero!(1, "Alpha", json!({}), json!({"signature1": "ability_dash"}))];
    let properties = json!({"AbilityCooldown": {"value": 25, "label": "Cooldown"}});
    let before = json!([
        {"property_upgrades": [{"name": "AbilityCooldown", "bonus": -12}]}
    ]);
    let after = json!([
        {"property_upgrades": [{"name": "AbilityCooldown", "bonus": -10}]},
        {"property_upgrades": [{"name": "Charges", "bonus": 1}]}
    ]);

    let patch = between(
        &heroes,
        &[ability!(10, "ability_dash", "Dash", properties, before)],
        &heroes,
        &[ability!(10, "ability_dash", "Dash", properties, after)],
    );

    assert_eq!(patch.heroes[0].abilities[0].changes, vec![
        change("T1 upgrade", Some("Cooldown -12"), Some("Cooldown -10")),
        change("T2 upgrade", None, Some("Charges +1")),
    ]);
}

#[test]
fn swapping_an_ability_slot_adds_and_removes() {
    let heroes =
        [hero!(1, "Alpha", json!({}), json!({"signature1": "ability_old"}))];
    let renamed =
        [hero!(1, "Alpha", json!({}), json!({"signature1": "ability_new"}))];

    let patch = between(
        &heroes,
        &[ability!(10, "ability_old", "Old Move", json!({}), json!([]))],
        &renamed,
        &[ability!(11, "ability_new", "New Move", json!({}), json!([]))],
    );

    let abilities = &patch.heroes[0].abilities;
    assert_eq!(abilities.len(), 2);
    assert_eq!((abilities[0].item_id, abilities[0].presence), (11, Presence::Added));
    assert_eq!(
        (abilities[1].item_id, abilities[1].presence),
        (10, Presence::Removed)
    );
    assert!(abilities.iter().all(|ability| ability.changes.is_empty()));
}

#[test]
fn heroes_entering_and_leaving_the_pool_are_added_and_removed() {
    let old = [unplayable!(1, "Alpha"), hero!(2, "Beta", json!({}), json!({}))];
    let new = [hero!(1, "Alpha", json!({"max_health": {"value": 800}}), json!({}))];

    let patch = between(&old, &[], &new, &[]);

    assert_eq!(patch.heroes.len(), 2);
    assert_eq!(
        (patch.heroes[0].hero_id, patch.heroes[0].presence),
        (1, Presence::Added)
    );
    assert_eq!(patch.heroes[0].stats, []);
    assert_eq!(
        (patch.heroes[1].hero_id, patch.heroes[1].presence),
        (2, Presence::Removed)
    );
}

#[test]
fn shop_items_report_cost_changes_additions_and_removals() {
    let old = [shop_item!(1, "Bullet Boost", 500), shop_item!(2, "Gone Item", 800)];
    let new =
        [shop_item!(1, "Bullet Boost", 800), shop_item!(3, "Fresh Item", 1200)];

    let patch = between(&[], &old, &[], &new);

    assert_eq!(patch.items.len(), 3);
    assert_eq!(patch.items[0].name, "Bullet Boost");
    assert_eq!(patch.items[0].presence, Presence::Changed);
    assert_eq!(patch.items[0].changes, vec![change(
        "Cost",
        Some("500"),
        Some("800")
    )]);
    assert_eq!(patch.items[1].presence, Presence::Added);
    assert_eq!(patch.items[2].presence, Presence::Removed);
}

#[test]
fn items_outside_the_shop_are_ignored() {
    let unshopable: Item = serde_json::from_value(json!({
        "id": 5, "class_name": "upgrade_hidden", "name": "Hidden", "type": "upgrade",
        "shopable": false, "cost": 100
    }))
    .unwrap();
    let mut changed = unshopable.clone();
    changed.cost = Some(200);

    let patch = between(
        &[],
        &[unshopable, ability!(6, "ability_x", "Skill", json!({}), json!([]))],
        &[],
        &[changed, weapon!("weapon_x", 10)],
    );

    assert_eq!(patch.items, []);
    assert!(patch.is_empty());
}

#[test]
fn hero_ids_follow_name_order() {
    let old = [
        hero!(5, "Zed", json!({"max_health": {"value": 1}}), json!({})),
        hero!(9, "Alpha", json!({"max_health": {"value": 1}}), json!({})),
    ];
    let new = [
        hero!(5, "Zed", json!({"max_health": {"value": 2}}), json!({})),
        hero!(9, "Alpha", json!({"max_health": {"value": 2}}), json!({})),
    ];

    let patch = between(&old, &[], &new, &[]);

    assert_eq!(patch.hero_ids(), vec![9, 5]);
    assert!(!patch.is_empty());
    assert!(PatchDiff::default().is_empty());
}

#[test]
fn a_patch_diff_survives_a_json_round_trip() {
    let old = [hero!(1, "Alpha", json!({"max_health": {"value": 830}}), json!({}))];
    let new = [hero!(1, "Alpha", json!({"max_health": {"value": 850}}), json!({}))];
    let patch = between(&old, &[], &new, &[]);

    let decoded: PatchDiff =
        serde_json::from_str(&serde_json::to_string(&patch).unwrap()).unwrap();

    assert_eq!(decoded, patch);
}
