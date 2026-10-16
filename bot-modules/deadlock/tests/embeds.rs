use deadlock::analysis::{
    Counters,
    ability_picks,
    build_ability_sequence,
    counters,
    item_phases,
    lane_summary,
};
use deadlock::embeds::{self, Freshness};
use deadlock::heroes::HeroIndex;
use deadlock::items::ItemIndex;
use deadlock_api::model::{
    AbilityOrderStats,
    Build,
    Hero,
    HeroCounterStats,
    HeroImages,
    Item,
    ItemFlowStats,
    LaneMatchupStats,
    LaneSoulCurve,
};
use jiff::Timestamp;
use serenity::all::CreateComponent;

const HEROES: &str = include_str!("fixtures/heroes.json");
const ITEMS: &str = include_str!("fixtures/items.json");
const ITEM_FLOW: &str = include_str!("fixtures/item_flow.json");
const ABILITY_ORDER: &str = include_str!("fixtures/ability_order.json");
const HERO_COUNTER: &str = include_str!("fixtures/hero_counter.json");
const LANE_MATCHUP: &str = include_str!("fixtures/lane_matchup.json");
const LANE_SOUL_CURVE: &str = include_str!("fixtures/lane_soul_curve.json");
const BUILDS: &str = include_str!("fixtures/builds.json");

fn heroes() -> serde_json::Result<HeroIndex> {
    let heroes: Vec<Hero> = serde_json::from_str(HEROES)?;
    Ok(HeroIndex::new(&heroes))
}

fn items() -> serde_json::Result<ItemIndex> {
    let items: Vec<Item> = serde_json::from_str(ITEMS)?;
    Ok(ItemIndex::new(&items))
}

fn enemy() -> Hero {
    Hero {
        id: 6,
        class_name: "hero_enemy".to_string(),
        name: "Enemy Laner".to_string(),
        player_selectable: true,
        disabled: false,
        in_development: false,
        images: HeroImages::default(),
    }
}

fn json(component: &CreateComponent<'_>) -> String {
    serde_json::to_string(component).unwrap_or_default()
}

const fn fresh(stale: bool) -> Freshness {
    Freshness { fetched_at: Timestamp::UNIX_EPOCH, stale }
}

#[test]
fn build_component_renders_phases_and_the_community_build() {
    let hero = heroes().unwrap().find("Infernus").cloned().unwrap();
    let items = items().unwrap();
    let flow: ItemFlowStats = serde_json::from_str(ITEM_FLOW).unwrap();
    let builds: Vec<Build> = serde_json::from_str(BUILDS).unwrap();
    let phases = item_phases(&flow);

    let component = embeds::build_component(
        &hero,
        Some("Oracle"),
        &phases,
        Some(&builds[0].hero_build),
        &items,
        &fresh(false),
    );
    let json = json(&component);

    assert!(json.contains("Infernus build"));
    assert!(json.contains("Oracle"));
    assert!(json.contains("Phase 1"));
    assert!(json.contains("Top community build"));
    assert!(json.contains("Extra Spirit"));
    assert!(json.contains("Improved Spirit"));
    assert!(json.contains("deadlock-api.com"));
    assert!(!json.contains("cached copy"));
}

#[test]
fn build_component_defaults_to_all_ranks() {
    let hero = heroes().unwrap().find("Infernus").cloned().unwrap();
    let items = items().unwrap();

    let component =
        embeds::build_component(&hero, None, &[], None, &items, &fresh(false));
    let json = json(&component);

    assert!(json.contains("All ranks"));
}

#[test]
fn build_component_degrades_without_data() {
    let hero = heroes().unwrap().find("Infernus").cloned().unwrap();
    let items = items().unwrap();

    let component =
        embeds::build_component(&hero, None, &[], None, &items, &fresh(false));
    let json = json(&component);

    assert!(json.contains("Infernus"));
    assert!(json.contains("unavailable"));
    assert!(json.contains("deadlock-api.com"));
}

#[test]
fn abilities_component_renders_both_picks_and_the_community_sequence() {
    let hero = heroes().unwrap().find("Infernus").cloned().unwrap();
    let items = items().unwrap();
    let orders: Vec<AbilityOrderStats> =
        serde_json::from_str(ABILITY_ORDER).unwrap();
    let builds: Vec<Build> = serde_json::from_str(BUILDS).unwrap();
    let picks = ability_picks(&orders).unwrap();
    let order = builds[0].hero_build.details.ability_order.as_ref().unwrap();
    let sequence = build_ability_sequence(&order.currency_changes);

    let component = embeds::abilities_component(
        &hero,
        None,
        Some(&picks),
        Some(("Community pick", sequence.as_slice())),
        &items,
        &fresh(false),
    );
    let json = json(&component);

    assert!(json.contains("Infernus ability order"));
    assert!(json.contains("Most played"));
    assert!(json.contains("Afterburn"));
    assert!(json.contains("Flame Dash"));
    assert!(json.contains("Top community build (Community pick)"));
    assert!(json.contains("deadlock-api.com"));
}

#[test]
fn abilities_component_degrades_without_data() {
    let hero = heroes().unwrap().find("Infernus").cloned().unwrap();
    let items = items().unwrap();

    let component =
        embeds::abilities_component(&hero, None, None, None, &items, &fresh(false));
    let json = json(&component);

    assert!(json.contains("Infernus"));
    assert!(json.contains("unavailable"));
    assert!(!json.contains("Top community build"));
}

#[test]
fn counter_component_lists_strong_and_weak_matchups() {
    let index = heroes().unwrap();
    let hero = index.find("Infernus").cloned().unwrap();
    let rows: Vec<HeroCounterStats> = serde_json::from_str(HERO_COUNTER).unwrap();
    let shown = counters(&rows, hero.id);

    let component = embeds::counter_component(&hero, &shown, &index, &fresh(false));
    let json = json(&component);

    assert!(json.contains("Infernus matchups"));
    assert!(json.contains("Strong against"));
    assert!(json.contains("Weak against"));
    assert!(json.contains("Hero #"));
    assert!(json.contains("deadlock-api.com"));
}

#[test]
fn counter_component_degrades_without_data() {
    let index = heroes().unwrap();
    let hero = index.find("Infernus").cloned().unwrap();
    let shown = Counters { strong: Vec::new(), weak: Vec::new() };

    let component = embeds::counter_component(&hero, &shown, &index, &fresh(false));
    let json = json(&component);

    assert!(json.contains("Infernus"));
    assert!(json.contains("unavailable"));
}

#[test]
fn matchup_component_renders_the_lane_table() {
    let hero = heroes().unwrap().find("Infernus").cloned().unwrap();
    let matchups: Vec<LaneMatchupStats> =
        serde_json::from_str(LANE_MATCHUP).unwrap();
    let curves: Vec<LaneSoulCurve> = serde_json::from_str(LANE_SOUL_CURVE).unwrap();
    let lane = lane_summary(&matchups, &curves, hero.id, 6).unwrap();

    let component = embeds::matchup_component(
        &hero,
        &enemy(),
        Some("Oracle"),
        Some(&lane),
        &fresh(false),
    );
    let json = json(&component);

    assert!(json.contains("Infernus vs Enemy Laner in lane"));
    assert!(json.contains("Oracle"));
    assert!(json.contains("Lane win rate"));
    assert!(json.contains("Soul lead over time"));
    assert!(json.contains("03:00"));
    assert!(json.contains("deadlock-api.com"));
}

#[test]
fn matchup_component_degrades_without_data() {
    let hero = heroes().unwrap().find("Infernus").cloned().unwrap();

    let component =
        embeds::matchup_component(&hero, &enemy(), None, None, &fresh(false));
    let json = json(&component);

    assert!(json.contains("Infernus"));
    assert!(json.contains("unavailable"));
}

#[test]
fn a_stale_source_flags_the_cached_copy() {
    let hero = heroes().unwrap().find("Infernus").cloned().unwrap();
    let items = items().unwrap();

    let component =
        embeds::build_component(&hero, None, &[], None, &items, &fresh(true));
    let json = json(&component);

    assert!(json.contains("cached copy"));
}

#[test]
fn combined_freshness_keeps_the_oldest_fetch_and_any_stale_flag() {
    let older = Timestamp::from_second(50).unwrap();
    let newer = Timestamp::from_second(100).unwrap();

    let combined = Freshness::combine(&[
        Freshness { fetched_at: newer, stale: false },
        Freshness { fetched_at: older, stale: true },
    ]);

    assert_eq!(combined.fetched_at, older);
    assert!(combined.stale);
}

#[test]
fn combining_no_sources_is_fresh() {
    let combined = Freshness::combine(&[]);

    assert!(!combined.stale);
}
