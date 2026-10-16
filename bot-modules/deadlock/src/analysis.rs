use std::collections::BTreeMap;

use deadlock_api::model::{
    AbilityOrderStats,
    CurrencyChange,
    HeroCounterStats,
    ItemFlowStats,
    LaneMatchupStats,
    LaneSoulCurve,
};

pub const ITEMS_PER_PHASE: usize = 6;
pub const MIN_PHASE_SHARE: f64 = 0.02;
pub const COUNTERS_SHOWN: usize = 5;
pub const MIN_COUNTER_MATCHES: u64 = 500;
pub const MIN_ORDER_SHARE: f64 = 0.10;

const ABILITY_POINT_CURRENCY: u8 = 1;

#[expect(
    clippy::cast_precision_loss,
    reason = "match counts stay far below 2^53, so the f64 conversion is exact"
)]
const fn to_f64(value: u64) -> f64 {
    value as f64
}

#[derive(Debug, Clone)]
pub struct RankedItem {
    pub item_id: u32,
    pub win_rate: f64,
    pub matches: u64,
}

#[derive(Debug, Clone)]
pub struct Phase {
    pub column: u8,
    pub items: Vec<RankedItem>,
}

#[must_use]
pub fn item_phases(flow: &ItemFlowStats) -> Vec<Phase> {
    let mut columns: Vec<u8> = flow.nodes.iter().map(|node| node.column).collect();
    columns.sort_unstable();
    columns.dedup();

    columns
        .into_iter()
        .filter_map(|column| {
            let reached = flow
                .reached_per_column
                .get(usize::from(column))
                .copied()
                .unwrap_or(flow.summary.matches);
            let floor = MIN_PHASE_SHARE * to_f64(reached);

            let mut items: Vec<RankedItem> = flow
                .nodes
                .iter()
                .filter(|node| {
                    node.column == column && to_f64(node.matches) >= floor
                })
                .map(|node| RankedItem {
                    item_id: node.item_id,
                    win_rate: node
                        .adjusted_win_rate
                        .unwrap_or_else(|| node.win_rate()),
                    matches: node.matches,
                })
                .collect();

            items.sort_by(|a, b| {
                b.win_rate
                    .total_cmp(&a.win_rate)
                    .then_with(|| b.matches.cmp(&a.matches))
            });
            items.truncate(ITEMS_PER_PHASE);

            (!items.is_empty()).then_some(Phase { column, items })
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct AbilityPicks<'a> {
    pub most_played: &'a AbilityOrderStats,
    pub best: &'a AbilityOrderStats,
}

#[must_use]
pub fn ability_picks(orders: &[AbilityOrderStats]) -> Option<AbilityPicks<'_>> {
    let most_played = orders.iter().max_by_key(|order| order.matches)?;
    let floor = MIN_ORDER_SHARE * to_f64(most_played.matches);

    let best = orders
        .iter()
        .filter(|order| to_f64(order.matches) >= floor)
        .max_by(|a, b| a.win_rate().total_cmp(&b.win_rate()))?;

    Some(AbilityPicks { most_played, best })
}

#[must_use]
pub fn build_ability_sequence(changes: &[CurrencyChange]) -> Vec<u32> {
    let sequence: Vec<u32> = changes
        .iter()
        .filter(|change| {
            change.currency_type == ABILITY_POINT_CURRENCY && change.delta < 0
        })
        .map(|change| change.ability_id)
        .collect();

    let (first, second) = sequence.split_at(sequence.len() / 2);
    if first.len() == second.len() && first == second {
        first.to_vec()
    } else {
        sequence
    }
}

#[derive(Debug, Clone)]
pub struct Counter {
    pub enemy_hero_id: u32,
    pub win_rate: f64,
    pub matches: u64,
}

#[derive(Debug, Clone)]
pub struct Counters {
    pub strong: Vec<Counter>,
    pub weak: Vec<Counter>,
}

#[must_use]
pub fn counters(rows: &[HeroCounterStats], hero_id: u32) -> Counters {
    let mut qualifying: Vec<Counter> = rows
        .iter()
        .filter(|row| {
            row.hero_id == hero_id
                && row.enemy_hero_id != hero_id
                && row.matches_played >= MIN_COUNTER_MATCHES
        })
        .map(|row| Counter {
            enemy_hero_id: row.enemy_hero_id,
            win_rate: row.win_rate(),
            matches: row.matches_played,
        })
        .collect();

    qualifying.sort_by(|a, b| {
        b.win_rate.total_cmp(&a.win_rate).then_with(|| b.matches.cmp(&a.matches))
    });

    let strong = qualifying.iter().take(COUNTERS_SHOWN).cloned().collect();
    let weak = qualifying.iter().rev().take(COUNTERS_SHOWN).cloned().collect();

    Counters { strong, weak }
}

#[derive(Debug, Clone)]
pub struct SoulPoint {
    pub time_s: u32,
    pub net_worth_diff: f64,
    pub matches: u64,
}

#[derive(Debug, Clone)]
pub struct LaneSummary {
    pub matches: u64,
    pub wins: u64,
    pub points: Vec<SoulPoint>,
}

impl LaneSummary {
    #[must_use]
    pub const fn win_rate(&self) -> f64 {
        if self.matches == 0 {
            return 0.0;
        }

        to_f64(self.wins) / to_f64(self.matches)
    }
}

#[must_use]
pub fn lane_summary(
    matchups: &[LaneMatchupStats],
    curves: &[LaneSoulCurve],
    hero_id: u32,
    enemy_id: u32,
) -> Option<LaneSummary> {
    let is_pair = |hero_ids: &[u32], enemy_hero_ids: &[u32]| {
        hero_ids.contains(&hero_id) && enemy_hero_ids.contains(&enemy_id)
    };

    let (matches, wins) = matchups
        .iter()
        .filter(|row| is_pair(&row.hero_ids, &row.enemy_hero_ids))
        .fold((0_u64, 0_u64), |(matches, wins), row| {
            (matches + row.matches_played, wins + row.wins)
        });

    let mut totals: BTreeMap<u32, (f64, u64)> = BTreeMap::new();
    for curve in curves.iter().filter(|c| is_pair(&c.hero_ids, &c.enemy_hero_ids)) {
        for (index, &time_s) in curve.sample_times_s.iter().enumerate() {
            let (Some(&weight), Some(&diff)) =
                (curve.sample_matches.get(index), curve.net_worth_diff.get(index))
            else {
                continue;
            };

            let total = totals.entry(time_s).or_insert((0.0, 0));
            total.0 = diff.mul_add(to_f64(weight), total.0);
            total.1 += weight;
        }
    }

    let points: Vec<SoulPoint> = totals
        .into_iter()
        .filter(|&(_, (_, weight))| weight > 0)
        .map(|(time_s, (weighted_sum, weight))| SoulPoint {
            time_s,
            net_worth_diff: weighted_sum / to_f64(weight),
            matches: weight,
        })
        .collect();

    if matches == 0 && points.is_empty() {
        return None;
    }

    Some(LaneSummary { matches, wins, points })
}
