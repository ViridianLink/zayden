use std::collections::HashSet;

use deadlock_api::model::{
    ItemKind,
    ItemStats,
    MatchMetadata,
    MatchPlayer,
    PerformancePoint,
    PlayerItem,
};
use jiff::Timestamp;

use crate::items::ItemIndex;

pub const SOUL_CHECKPOINTS_S: [u32; 4] = [360, 900, 1800, 2700];
pub const ITEM_TIMINGS_SHOWN: usize = 6;
pub const MIN_ITEM_MATCHES: u64 = 100;

#[derive(Debug, Clone, Default)]
pub struct HeroAverages {
    pub curve: Vec<PerformancePoint>,
    pub items: Vec<ItemStats>,
    pub most_played_order: Option<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoulCheckpoint {
    pub time_s: u32,
    pub net_worth: u32,
    pub average: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemTiming {
    pub item_id: u32,
    pub bought_at_s: u32,
    pub average_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityComparison {
    pub followed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchSummary {
    pub match_id: u64,
    pub account_id: u32,
    pub hero_id: u32,
    pub won: bool,
    pub start_time: Timestamp,
    pub duration_s: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub net_worth: u32,
    pub last_hits: u32,
    pub denies: u32,
    pub level: u32,
    pub souls: Vec<SoulCheckpoint>,
    pub items: Vec<ItemTiming>,
    pub abilities: Option<AbilityComparison>,
}

fn ordered<'a>(
    player: &'a MatchPlayer,
    items: &ItemIndex,
    kind: ItemKind,
) -> Vec<&'a PlayerItem> {
    let mut owned: Vec<&PlayerItem> = player
        .items
        .iter()
        .filter(|owned| {
            items.get(owned.item_id).is_some_and(|item| item.kind == kind)
        })
        .collect();
    owned.sort_by_key(|owned| owned.game_time_s);

    owned
}

fn souls(
    meta: &MatchMetadata,
    player: &MatchPlayer,
    averages: &HeroAverages,
) -> Vec<SoulCheckpoint> {
    SOUL_CHECKPOINTS_S
        .iter()
        .copied()
        .filter(|time_s| *time_s <= meta.duration_s)
        .filter_map(|time_s| {
            let sample =
                player.stats.iter().find(|sample| sample.time_stamp_s == time_s)?;
            let average = averages
                .curve
                .iter()
                .find(|point| point.game_time == time_s)
                .map(|point| point.net_worth_avg);

            Some(SoulCheckpoint { time_s, net_worth: sample.net_worth, average })
        })
        .collect()
}

fn timings(
    player: &MatchPlayer,
    averages: &HeroAverages,
    items: &ItemIndex,
) -> Vec<ItemTiming> {
    let mut seen = HashSet::new();

    ordered(player, items, ItemKind::Upgrade)
        .into_iter()
        .filter(|owned| seen.insert(owned.item_id))
        .take(ITEM_TIMINGS_SHOWN)
        .map(|owned| {
            let average_s = averages
                .items
                .iter()
                .find(|stats| {
                    stats.item_id == owned.item_id
                        && stats.matches >= MIN_ITEM_MATCHES
                })
                .and_then(|stats| stats.avg_buy_time_s);

            ItemTiming {
                item_id: owned.item_id,
                bought_at_s: owned.game_time_s,
                average_s,
            }
        })
        .collect()
}

#[must_use]
pub fn ability_sequence(player: &MatchPlayer, items: &ItemIndex) -> Vec<u32> {
    ordered(player, items, ItemKind::Ability)
        .into_iter()
        .map(|owned| owned.item_id)
        .collect()
}

fn compare_abilities(
    player: &MatchPlayer,
    averages: &HeroAverages,
    items: &ItemIndex,
) -> Option<AbilityComparison> {
    let order = averages.most_played_order.as_deref()?;
    let sequence = ability_sequence(player, items);
    if sequence.is_empty() {
        return None;
    }

    Some(AbilityComparison {
        followed: sequence
            .iter()
            .zip(order)
            .take_while(|(played, expected)| played == expected)
            .count(),
        total: sequence.len().min(order.len()),
    })
}

#[must_use]
pub fn summarize(
    meta: &MatchMetadata,
    player: &MatchPlayer,
    averages: &HeroAverages,
    items: &ItemIndex,
) -> MatchSummary {
    MatchSummary {
        match_id: meta.match_id,
        account_id: player.account_id,
        hero_id: player.hero_id,
        won: player.won(&meta.winning_team),
        start_time: meta.start_time,
        duration_s: meta.duration_s,
        kills: player.kills,
        deaths: player.deaths,
        assists: player.assists,
        net_worth: player.net_worth,
        last_hits: player.last_hits,
        denies: player.denies,
        level: player.player_level,
        souls: souls(meta, player, averages),
        items: timings(player, averages, items),
        abilities: compare_abilities(player, averages, items),
    }
}
