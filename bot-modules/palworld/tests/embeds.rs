use palworld::embeds;
use palworld::model::{Element, Item, Pal, Stats, Suitability};

fn lamball() -> Pal {
    Pal {
        key: "001".to_string(),
        paldex_no: 1,
        name: "Lamball".to_string(),
        elements: vec![Element::Neutral],
        stats: Some(Stats { hp: 70, attack_melee: 70, ..Stats::default() }),
        suitability: vec![Suitability { kind: "handiwork".to_string(), level: 1 }],
        drops: vec!["wool".to_string()],
        breeding_rank: Some(1470),
        ..Pal::default()
    }
}

fn render(component: &serenity::all::CreateComponent<'_>) -> String {
    serde_json::to_string(component).unwrap_or_default()
}

#[test]
fn pal_component_includes_name_and_stats() {
    let json = render(&embeds::pal_component(&lamball()));
    assert!(json.contains("Lamball"));
    assert!(json.contains("Handiwork") || json.contains("handiwork"));
    assert!(json.contains("Wool") || json.contains("wool"));
}

#[test]
fn breeding_component_shows_parents_and_child() {
    let a = lamball();
    let mut b = lamball();
    b.name = "Cattiva".to_string();
    let mut child = lamball();
    child.name = "Chikipi".to_string();

    let json = render(&embeds::breeding_component(&a, &b, &child, true));
    assert!(json.contains("Lamball"));
    assert!(json.contains("Cattiva"));
    assert!(json.contains("Chikipi"));
    assert!(json.contains("Special combination"));
}

#[test]
fn type_component_lists_effectiveness() {
    let json = render(&embeds::type_component(
        Element::Fire,
        &[Element::Grass, Element::Ice],
        &[Element::Water],
        &["Foxparks".to_string()],
    ));
    assert!(json.contains("Fire"));
    assert!(json.contains("Grass"));
    assert!(json.contains("Water"));
    assert!(json.contains("Foxparks"));
}

#[test]
fn item_component_renders() {
    let item = Item {
        key: "gold_coin".to_string(),
        name: "Gold Coin".to_string(),
        gold: Some(1),
        ..Item::default()
    };
    let json = render(&embeds::item_component(&item));
    assert!(json.contains("Gold Coin"));
}
