use palworld::embeds;
use palworld::model::{Element, Item, Pal, Stats, Suitability};
use palworld::progress::Region;

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
fn link_component_shared_world_omits_host() {
    let json = render(&embeds::link_component("Bob", 12, None));
    assert!(json.contains("Bob"));
    assert!(json.contains("12 breedable Pals"));
    assert!(!json.contains("uploaded world"));
}

#[test]
fn link_component_names_host_world() {
    let json = render(&embeds::link_component("Bob", 12, Some("<@42>")));
    assert!(json.contains("Bob"));
    assert!(json.contains("<@42>"));
    assert!(json.contains("uploaded world"));
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

/// A player who has explored 87 Palpagos statues, cleared every tower on both
/// maps, and reached the World Tree.
fn sample_progress() -> palworld::progress::Progress {
    sample_with_maps(["MainMap", "Tree"])
}

fn sample_with_maps<const N: usize>(
    maps: [&str; N],
) -> palworld::progress::Progress {
    let cat = palworld::progress::catalogue();
    let record = palworld::save::player::PlayerRecord {
        fast_travel: cat
            .fast_travel_on(Region::Palpagos)
            .take(87)
            .map(|p| p.id.clone())
            .collect(),
        towers_defeated: cat.towers.iter().map(|t| t.flag.clone()).collect(),
        world_maps: maps.iter().map(|s| (*s).to_string()).collect(),
        notes: ["Day1", "Day2"].iter().map(|s| (*s).to_string()).collect(),
        normal_dungeons_cleared: 4,
        ..palworld::save::player::PlayerRecord::default()
    };
    let roster = palworld::model::PlayerRoster {
        name: "Oscar Six".to_string(),
        level: 45,
        ..palworld::model::PlayerRoster::default()
    };
    palworld::progress::compute(Some(&record), &roster, cat)
}

#[test]
fn progress_component_shows_bars_and_the_ownership_caveat() {
    let progress = sample_progress();
    let json = render(&embeds::progress_component(&progress));

    assert!(json.contains("Oscar Six"));
    assert!(json.contains("Level 45"));
    // Half-filled and fully-filled bars both render.
    assert!(json.contains("▰"), "a filled bar cell");
    assert!(json.contains("▱"), "an empty bar cell");
    assert!(json.contains("87/157"), "Palpagos fast travel ratio");
    assert!(json.contains("9/9"), "a completed milestone");
    assert!(json.contains("✅"), "completion is marked");
    // Uncapped counters are separated out, not mixed into the ranked list.
    assert!(json.contains("Also tracked"));
    assert!(json.contains("Lore notes found"));
    // The rule that progression excludes guild Pals must be stated on-screen.
    assert!(json.contains("guild"), "ownership caveat is shown");
}

/// The overview must never present a World Tree total as part of a Palpagos one
/// - that is what sent players hunting a statue in open water.
#[test]
fn progress_component_separates_the_two_maps() {
    let json = render(&embeds::progress_component(&sample_progress()));

    assert!(json.contains("Palpagos Islands"), "Palpagos heading: {json}");
    assert!(json.contains("World Tree"), "World Tree heading: {json}");
    assert!(json.contains("Across both maps"), "map-independent heading");
    // The pooled 174 must not appear anywhere.
    assert!(!json.contains("/174"), "maps are still pooled: {json}");
    // Both halves are present with their own denominators.
    assert!(json.contains("87/157"), "Palpagos fast travel");
    assert!(json.contains("0/17"), "World Tree fast travel");
    assert!(json.contains("4/4"), "World Tree towers");
    // This player has reached the World Tree, so no lock notice.
    assert!(!json.contains("Not discovered"), "unlocked map is not flagged");
}

#[test]
fn progress_component_flags_a_map_the_player_has_not_reached() {
    let json = render(&embeds::progress_component(&sample_with_maps(["MainMap"])));

    assert!(json.contains("Not discovered"), "lock notice: {json}");
}

#[test]
fn progress_detail_lists_missing_entries_with_map_coordinates() {
    let progress = sample_progress();
    let milestone = progress.milestone("fast-travel").expect("milestone");
    let json = render(&embeds::progress_detail_component(&progress, milestone));

    assert!(json.contains("Fast travel points"));
    assert!(json.contains("Palpagos Islands"), "the map is named: {json}");
    assert!(json.contains("Still missing"));
    assert!(json.contains("map coordinates"));
    // The list is capped, so a long tail is summarised rather than dumped.
    assert!(json.contains("and 45 more"), "tail summary: {json}");
}

#[test]
fn progress_detail_says_so_when_a_milestone_is_finished() {
    let progress = sample_progress();
    let milestone = progress.milestone("tree-towers").expect("milestone");
    let json = render(&embeds::progress_detail_component(&progress, milestone));

    assert!(json.contains("4/4"));
    assert!(json.contains("World Tree"));
    assert!(json.contains("Nothing left"));
}
