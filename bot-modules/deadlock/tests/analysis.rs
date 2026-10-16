use std::collections::BTreeSet;

use deadlock::analysis::{
    COUNTERS_SHOWN,
    ITEMS_PER_PHASE,
    MIN_COUNTER_MATCHES,
    ability_picks,
    build_ability_sequence,
    counters,
    item_phases,
    lane_summary,
};
use deadlock_api::model::{
    AbilityOrderStats,
    Build,
    CurrencyChange,
    FlowSummary,
    HeroCounterStats,
    ItemFlowNode,
    ItemFlowStats,
    LaneMatchupStats,
    LaneSoulCurve,
};

const ITEM_FLOW: &str = include_str!("fixtures/item_flow.json");
const ABILITY_ORDER: &str = include_str!("fixtures/ability_order.json");
const HERO_COUNTER: &str = include_str!("fixtures/hero_counter.json");
const LANE_MATCHUP: &str = include_str!("fixtures/lane_matchup.json");
const LANE_SOUL_CURVE: &str = include_str!("fixtures/lane_soul_curve.json");
const BUILDS: &str = include_str!("fixtures/builds.json");

const AFTERBURN: u32 = 1_593_133_799;

fn item_flow() -> serde_json::Result<ItemFlowStats> {
    serde_json::from_str(ITEM_FLOW)
}

fn ability_orders() -> serde_json::Result<Vec<AbilityOrderStats>> {
    serde_json::from_str(ABILITY_ORDER)
}

fn hero_counters() -> serde_json::Result<Vec<HeroCounterStats>> {
    serde_json::from_str(HERO_COUNTER)
}

fn lane_matchups() -> serde_json::Result<Vec<LaneMatchupStats>> {
    serde_json::from_str(LANE_MATCHUP)
}

fn lane_curves() -> serde_json::Result<Vec<LaneSoulCurve>> {
    serde_json::from_str(LANE_SOUL_CURVE)
}

fn builds() -> serde_json::Result<Vec<Build>> {
    serde_json::from_str(BUILDS)
}

const fn node(
    column: u8,
    item_id: u32,
    wins: u64,
    matches: u64,
    adjusted_win_rate: Option<f64>,
) -> ItemFlowNode {
    ItemFlowNode {
        column,
        item_id,
        wins,
        losses: matches - wins,
        matches,
        adjusted_win_rate,
        avg_net_worth_at_buy: None,
    }
}

const fn change(ability_id: u32, currency_type: u8, delta: i32) -> CurrencyChange {
    CurrencyChange { ability_id, currency_type, delta }
}

const fn counter_row(
    hero_id: u32,
    enemy_hero_id: u32,
    wins: u64,
    matches_played: u64,
) -> HeroCounterStats {
    HeroCounterStats {
        hero_id,
        enemy_hero_id,
        wins,
        matches_played,
        kills: 0,
        deaths: 0,
        enemy_kills: 0,
        enemy_deaths: 0,
    }
}

fn curve(times: Vec<u32>, weights: Vec<u64>, diffs: Vec<f64>) -> LaneSoulCurve {
    LaneSoulCurve {
        assigned_lane: 1,
        hero_ids: vec![1, 2],
        enemy_hero_ids: vec![6],
        sample_times_s: times,
        sample_matches: weights,
        matches_played: 0,
        net_worth_diff: diffs,
        net_worth_diff_std: Vec::new(),
    }
}

#[test]
fn item_phases_are_ascending_ranked_and_above_the_share_floor() {
    let flow = item_flow().unwrap();

    let phases = item_phases(&flow);

    assert!(!phases.is_empty());
    assert!(phases.windows(2).all(|pair| pair[0].column < pair[1].column));

    for phase in &phases {
        assert!(!phase.items.is_empty());
        assert!(phase.items.len() <= ITEMS_PER_PHASE);
        assert!(
            phase.items.windows(2).all(|pair| pair[0].win_rate >= pair[1].win_rate)
        );

        let reached = flow
            .reached_per_column
            .get(usize::from(phase.column))
            .copied()
            .unwrap_or(flow.summary.matches);
        for item in &phase.items {
            assert!(
                item.matches * 50 >= reached,
                "item {} in column {} is below the share floor",
                item.item_id,
                phase.column
            );
        }
    }
}

#[test]
fn item_phases_fall_back_to_the_summary_when_no_column_totals_exist() {
    let flow = ItemFlowStats {
        nodes: vec![
            node(0, 1, 15, 30, None),
            node(0, 2, 5, 10, None),
            node(0, 3, 90, 100, Some(0.2)),
        ],
        edges: Vec::new(),
        summary: FlowSummary { wins: 500, losses: 500, matches: 1_000 },
        reached_per_column: Vec::new(),
    };

    let phases = item_phases(&flow);

    assert_eq!(phases.len(), 1);
    let ids: Vec<u32> = phases[0].items.iter().map(|item| item.item_id).collect();
    assert_eq!(ids, [1, 3]);
}

#[test]
fn item_phases_skip_columns_with_no_qualifying_items() {
    let flow = ItemFlowStats {
        nodes: vec![node(0, 1, 50, 100, None), node(2, 2, 1, 2, None)],
        edges: Vec::new(),
        summary: FlowSummary { wins: 500, losses: 500, matches: 1_000 },
        reached_per_column: vec![1_000, 1_000, 1_000],
    };

    let phases = item_phases(&flow);

    let columns: Vec<u8> = phases.iter().map(|phase| phase.column).collect();
    assert_eq!(columns, [0]);
}

#[test]
fn ability_picks_choose_the_most_played_and_a_qualifying_best() {
    let orders = ability_orders().unwrap();

    let picks = ability_picks(&orders).unwrap();

    let max = orders.iter().map(|order| order.matches).max().unwrap();
    assert_eq!(picks.most_played.matches, max);
    assert!(picks.best.matches * 10 >= max);
    assert!(
        orders
            .iter()
            .filter(|order| order.matches * 10 >= max)
            .all(|order| order.win_rate() <= picks.best.win_rate())
    );
}

#[test]
fn ability_picks_are_none_without_orders() {
    assert!(ability_picks(&[]).is_none());
}

#[test]
fn a_published_build_sequence_drops_its_repeated_half() {
    let builds = builds().unwrap();
    let order = builds[0].hero_build.details.ability_order.as_ref().unwrap();

    let sequence = build_ability_sequence(&order.currency_changes);

    assert_eq!(sequence.len(), 12);
    assert_eq!(sequence[0], AFTERBURN);
}

#[test]
fn a_non_repeating_sequence_is_unchanged() {
    let changes = [
        change(1, 1, -1),
        change(1, 2, -1),
        change(2, 1, -1),
        change(3, 1, 1),
        change(3, 1, -2),
    ];

    assert_eq!(build_ability_sequence(&changes), [1, 2, 3]);
}

#[test]
fn an_even_non_repeating_sequence_is_unchanged() {
    let changes =
        [change(1, 1, -1), change(2, 1, -1), change(3, 1, -1), change(4, 1, -1)];

    assert_eq!(build_ability_sequence(&changes), [1, 2, 3, 4]);
}

#[test]
fn counters_rank_the_fixture_rows_for_the_hero() {
    let rows = hero_counters().unwrap();

    let result = counters(&rows, 1);

    assert!(!result.strong.is_empty());
    assert!(!result.weak.is_empty());
    assert!(result.strong.len() <= COUNTERS_SHOWN);
    assert!(result.weak.len() <= COUNTERS_SHOWN);
    assert!(
        result.strong.windows(2).all(|pair| pair[0].win_rate >= pair[1].win_rate)
    );
    assert!(result.weak.windows(2).all(|pair| pair[0].win_rate <= pair[1].win_rate));

    for counter in result.strong.iter().chain(&result.weak) {
        assert_ne!(counter.enemy_hero_id, 1);
        assert!(counter.matches >= MIN_COUNTER_MATCHES);
    }
}

#[test]
fn counters_ignore_other_heroes_self_rows_and_thin_samples() {
    let rows = [
        counter_row(1, 2, 60, 100_000),
        counter_row(1, 3, 40, 100_000),
        counter_row(1, 1, 99, 100_000),
        counter_row(1, 4, 5, 10),
        counter_row(9, 5, 70, 100_000),
    ];

    let result = counters(&rows, 1);

    let strong: Vec<u32> = result.strong.iter().map(|c| c.enemy_hero_id).collect();
    let weak: Vec<u32> = result.weak.iter().map(|c| c.enemy_hero_id).collect();
    assert_eq!(strong, [2, 3]);
    assert_eq!(weak, [3, 2]);
    assert!(counters(&rows, 77).strong.is_empty());
}

#[test]
fn lane_summary_totals_the_fixture_matchups() {
    let matchups = lane_matchups().unwrap();
    let curves = lane_curves().unwrap();

    let summary = lane_summary(&matchups, &curves, 1, 6).unwrap();

    let expected: u64 = matchups
        .iter()
        .filter(|row| row.hero_ids.contains(&1) && row.enemy_hero_ids.contains(&6))
        .map(|row| row.matches_played)
        .sum();
    assert!(expected > 0);
    assert_eq!(summary.matches, expected);
    assert!((0.0..=1.0).contains(&summary.win_rate()));

    assert!(!summary.points.is_empty());
    assert!(summary.points.windows(2).all(|pair| pair[0].time_s < pair[1].time_s));

    let sampled: BTreeSet<u32> =
        curves.iter().flat_map(|c| c.sample_times_s.iter().copied()).collect();
    assert!(summary.points.iter().all(|point| sampled.contains(&point.time_s)));
}

#[test]
fn lane_summary_is_none_for_unrelated_heroes() {
    let matchups = lane_matchups().unwrap();
    let curves = lane_curves().unwrap();

    assert!(lane_summary(&matchups, &curves, 998, 999).is_none());
    assert!(lane_summary(&matchups, &curves, 1, 999).is_none());
}

#[test]
fn soul_points_are_weighted_by_sample_matches() {
    let curves = [
        curve(vec![360, 180], vec![1, 1], vec![100.0, 10.0]),
        curve(vec![180, 360], vec![3, 3], vec![400.0, 700.0]),
    ];

    let summary = lane_summary(&[], &curves, 1, 6).unwrap();

    let times: Vec<u32> = summary.points.iter().map(|point| point.time_s).collect();
    assert_eq!(times, [180, 360]);
    assert_eq!(summary.points[0].matches, 4);
    assert!((summary.points[0].net_worth_diff - 302.5).abs() < 1e-9);
    assert!((summary.points[1].net_worth_diff - 550.0).abs() < 1e-9);
    assert!(summary.win_rate().abs() < f64::EPSILON);
}

#[test]
fn soul_samples_missing_a_parallel_entry_are_skipped() {
    let curves = [
        curve(vec![180, 360, 540], vec![2, 0], vec![50.0, 70.0, 90.0]),
        curve(vec![180, 360], vec![0, 0], vec![1.0, 2.0]),
    ];

    let summary = lane_summary(&[], &curves, 1, 6).unwrap();

    let times: Vec<u32> = summary.points.iter().map(|point| point.time_s).collect();
    assert_eq!(times, [180]);
}
