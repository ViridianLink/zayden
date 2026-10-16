use deadlock::heroes::HeroIndex;
use deadlock_api::model::Hero;

const HEROES: &str = include_str!("fixtures/heroes.json");

fn index() -> serde_json::Result<HeroIndex> {
    let heroes: Vec<Hero> = serde_json::from_str(HEROES)?;
    Ok(HeroIndex::new(&heroes))
}

fn names(heroes: &[&Hero]) -> Vec<String> {
    heroes.iter().map(|hero| hero.name.clone()).collect()
}

#[test]
fn the_index_drops_disabled_and_unreleased_heroes() {
    let index = index().unwrap();

    assert!(index.get(999).is_none());
    assert!(index.iter().all(|hero| hero.id != 999));
    assert!(index.get(1).is_some());
}

#[test]
fn heroes_are_sorted_by_name() {
    let index = index().unwrap();

    let names: Vec<&str> = index.iter().map(|hero| hero.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}

#[test]
fn find_resolves_ids_names_and_class_names() {
    let index = index().unwrap();

    assert_eq!(index.find("1").unwrap().name, "Infernus");
    assert_eq!(index.find(" infernus ").unwrap().id, 1);
    assert_eq!(index.find("hero_inferno").unwrap().id, 1);
    assert_eq!(index.find("inferno").unwrap().id, 1);
    assert!(index.find("999").is_none());
    assert!(index.find("nobody").is_none());
}

#[test]
fn search_lists_prefix_matches_before_substring_matches() {
    let index = index().unwrap();

    assert_eq!(names(&index.search("s", 25)), ["Seven", "Abrams", "Infernus"]);
    assert_eq!(names(&index.search("S", 2)), ["Seven", "Abrams"]);
    assert_eq!(names(&index.search("in", 25)), ["Infernus"]);
}

#[test]
fn an_empty_search_lists_heroes_by_name() {
    let index = index().unwrap();

    assert_eq!(names(&index.search("", 2)), ["Abrams", "Infernus"]);
}
