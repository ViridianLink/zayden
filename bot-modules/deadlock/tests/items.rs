use std::collections::BTreeMap;

use deadlock::items::ItemIndex;
use deadlock_api::model::{Item, ItemKind, WeaponInfo};

const ITEMS: &str = include_str!("fixtures/items.json");

fn item(id: u32, class_name: &str, name: &str) -> Item {
    Item {
        id,
        class_name: class_name.to_string(),
        name: name.to_string(),
        kind: ItemKind::Upgrade,
        item_slot_type: None,
        item_tier: None,
        cost: None,
        shopable: true,
        image: None,
        hero: None,
        heroes: Vec::new(),
        ability_type: None,
        properties: BTreeMap::new(),
        upgrades: Vec::new(),
        weapon_info: WeaponInfo::default(),
    }
}

#[test]
fn names_resolve_from_the_fixture() {
    let items: Vec<Item> = serde_json::from_str(ITEMS).unwrap();
    let index = ItemIndex::new(&items);

    assert_eq!(index.name(1_593_133_799), "Afterburn");
    assert!(index.get(1_593_133_799).is_some());
}

#[test]
fn a_blank_name_falls_back_to_the_class_name_then_the_id() {
    let index = ItemIndex::new(&[
        item(1, "upgrade_named", "Named"),
        item(2, "upgrade_unnamed", ""),
        item(3, "", ""),
    ]);

    assert_eq!(index.name(1), "Named");
    assert_eq!(index.name(2), "upgrade_unnamed");
    assert_eq!(index.name(3), "#3");
    assert_eq!(index.name(4), "#4");
    assert!(index.get(4).is_none());
}
