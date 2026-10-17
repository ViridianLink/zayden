use std::collections::BTreeMap;

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
use deadlock::summary::{
    AbilityComparison,
    ItemTiming,
    MatchSummary,
    SoulCheckpoint,
};
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
        starting_stats: BTreeMap::new(),
        items: BTreeMap::new(),
        standard_level_up_upgrades: BTreeMap::new(),
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

fn summary(hero_id: u32, won: bool, average: bool) -> MatchSummary {
    MatchSummary {
        match_id: 69_192_735,
        account_id: 141_610_988,
        hero_id,
        won,
        start_time: Timestamp::UNIX_EPOCH,
        duration_s: 749,
        kills: 9,
        deaths: 1,
        assists: 22,
        net_worth: 37_306,
        last_hits: 205,
        denies: 2,
        level: 29,
        souls: vec![SoulCheckpoint {
            time_s: 360,
            net_worth: 5_486,
            average: average.then_some(5_000.0),
        }],
        items: vec![ItemTiming {
            item_id: 968_099_481,
            bought_at_s: 125,
            average_s: average.then_some(100.0),
        }],
        abilities: average.then_some(AbilityComparison { followed: 5, total: 7 }),
    }
}

#[test]
fn summary_component_shows_the_outcome_and_scoreline() {
    let index = heroes().unwrap();
    let hero = index.find("Infernus").unwrap();
    let items = items().unwrap();

    let won = embeds::summary_component(
        &summary(hero.id, true, true),
        Some(hero),
        &items,
        None,
    );
    let lost = embeds::summary_component(
        &summary(hero.id, false, true),
        Some(hero),
        &items,
        None,
    );
    let won = json(&won);

    assert!(won.contains("Victory"));
    assert!(json(&lost).contains("Defeat"));
    assert!(won.contains("as Infernus"));
    assert!(won.contains("K/D/A 9/1/22"));
    assert!(won.contains("37306 souls"));
    assert!(won.contains("205 LH / 2 DN"));
    assert!(won.contains("12:29"));
    assert!(won.contains("<t:0:R>"));
    assert!(!won.contains("deadlock-api.com"));
}

#[test]
fn summary_component_compares_against_the_averages() {
    let items = items().unwrap();

    let component =
        embeds::summary_component(&summary(1, true, true), None, &items, None);
    let json = json(&component);

    assert!(json.contains("as Hero #1"));
    assert!(json.contains("6m  5486  (+486 vs avg)"));
    assert!(json.contains("Extra Spirit 2:05 (+0:25 vs avg)"));
    assert!(
        json.contains("Followed the most-played ability order for 5/7 upgrades")
    );
}

#[test]
fn summary_component_omits_comparisons_without_averages() {
    let items = items().unwrap();

    let component =
        embeds::summary_component(&summary(1, true, false), None, &items, None);
    let json = json(&component);

    assert!(json.contains("6m  5486"));
    assert!(json.contains("Extra Spirit 2:05"));
    assert!(!json.contains("vs avg"));
    assert!(!json.contains("most-played"));
}

#[test]
fn summary_post_mentions_the_player_without_pinging() {
    let index = heroes().unwrap();
    let items = items().unwrap();
    let first = summary(1, true, true);
    let second = summary(1, false, false);

    let message =
        embeds::summary_post(&[(&first, 123), (&second, 456)], &index, &items);
    let json = serde_json::to_string(&message).unwrap_or_default();

    assert!(json.contains("<@123>"));
    assert!(json.contains("<@456>"));
    assert!(json.contains("Victory"));
    assert!(json.contains("Defeat"));
    assert!(json.contains("allowed_mentions"));
}

#[test]
fn summary_dm_is_a_components_message_without_a_mention() {
    let items = items().unwrap();

    let message = embeds::summary_dm(&summary(1, true, true), None, &items);
    let json = serde_json::to_string(&message).unwrap_or_default();

    assert!(json.contains("Victory"));
    assert!(!json.contains("<@"));
    assert!(json.contains("allowed_mentions"));
}
