use deadlock::autocomplete::{hero_choices, rank_choices};
use deadlock::heroes::HeroIndex;
use deadlock_api::model::{Hero, Rank};
use serde_json::Value;
use serenity::all::AutocompleteChoice;
use zayden_core::limits::autocomplete::CHOICES as MAX_CHOICES;

const HEROES: &str = include_str!("fixtures/heroes.json");
const RANKS: &str = include_str!("fixtures/ranks.json");

fn heroes() -> serde_json::Result<HeroIndex> {
    let heroes: Vec<Hero> = serde_json::from_str(HEROES)?;
    Ok(HeroIndex::new(&heroes))
}

fn ranks() -> serde_json::Result<Vec<Rank>> {
    serde_json::from_str(RANKS)
}

fn pairs(choices: &[AutocompleteChoice<'_>]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|choice| {
            let value = serde_json::to_value(choice).unwrap_or_default();
            let field = |key: &str| {
                value
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            };
            (field("name"), field("value"))
        })
        .collect()
}

#[test]
fn an_empty_hero_query_lists_names_in_order() {
    let index = heroes().unwrap();

    let choices = pairs(&hero_choices(&index, ""));

    assert_ne!(choices, Vec::<(String, String)>::new());
    assert!(choices.len() <= MAX_CHOICES);
    assert!(choices.windows(2).all(|pair| pair[0].0 <= pair[1].0));
    assert!(choices.iter().any(|(name, _)| name == "Infernus"));
}

#[test]
fn hero_prefix_matches_come_first_and_values_are_ids() {
    let index = heroes().unwrap();

    let choices = pairs(&hero_choices(&index, "in"));

    assert_eq!(choices[0], ("Infernus".to_string(), "1".to_string()));
    assert!(choices.iter().all(|(_, value)| value.parse::<u32>().is_ok()));

    let by_s = pairs(&hero_choices(&index, "s"));
    assert_eq!(by_s[0].0, "Seven");
}

#[test]
fn an_unmatched_hero_query_returns_nothing() {
    let index = heroes().unwrap();

    assert!(hero_choices(&index, "zzzz").is_empty());
}

#[test]
fn rank_choices_exclude_obscurus_and_follow_tier_order() {
    let ranks = ranks().unwrap();

    let choices = pairs(&rank_choices(&ranks, ""));

    assert!(choices.len() <= MAX_CHOICES);
    assert!(choices.iter().all(|(name, _)| name != "Obscurus"));
    assert!(choices.iter().all(|(_, value)| value != "0"));
    let tiers: Vec<u8> =
        choices.iter().filter_map(|(_, value)| value.parse().ok()).collect();
    assert_eq!(tiers.len(), choices.len());
    assert!(tiers.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn rank_choices_filter_by_case_insensitive_substring() {
    let ranks = ranks().unwrap();

    let choices = pairs(&rank_choices(&ranks, "ORACLE"));

    assert_eq!(choices, [("Oracle".to_string(), "8".to_string())]);
    assert!(rank_choices(&ranks, "obscurus").is_empty());
}
