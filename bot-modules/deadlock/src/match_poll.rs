use std::collections::{BTreeSet, HashMap, HashSet};

use deadlock_api::model::{MatchMetadata, MatchPlayer};
use deadlock_api::{DeadlockApiError, Priority};
use jiff::{SignedDuration, Timestamp};
use sqlx::PgPool;
use tracing::{debug, warn};

use crate::analysis;
use crate::client::{DeadlockApi, Fetched};
use crate::db::matches::{self, SeenMatch};
use crate::db::players::{self, PolledPlayer};
use crate::error::{DeadlockError, Result};
use crate::items::ItemIndex;
use crate::summary::{self, HeroAverages, MatchSummary};

pub const LOOKBACK: SignedDuration = SignedDuration::from_hours(3);

#[derive(Debug, Clone)]
pub struct SummaryJob {
    pub discord_id: i64,
    pub summary: MatchSummary,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PollOutcome {
    pub players: usize,
    pub matches: usize,
    pub new: usize,
}

#[must_use]
pub const fn next_streak(streak: i32, won: bool) -> i32 {
    if won {
        let base = if streak > 0 { streak } else { 0 };
        base.saturating_add(1)
    } else {
        let base = if streak < 0 { streak } else { 0 };
        base.saturating_sub(1)
    }
}

fn settle<T: Clone + Default>(
    result: Result<Fetched<T>>,
    what: &'static str,
    hero_id: u32,
) -> T {
    match result {
        Ok(fetched) => (*fetched.value).clone(),
        Err(DeadlockError::Api(e @ DeadlockApiError::Busy { .. })) => {
            debug!(error = %e, hero_id, what, "deadlock: summary average deferred, deadlock-api is busy");
            T::default()
        },
        Err(e) => {
            warn!(error = ?e, hero_id, what, "deadlock: summary average unavailable");
            T::default()
        },
    }
}

async fn hero_averages(api: &DeadlockApi, hero_id: u32) -> HeroAverages {
    let (curve, items, orders) = futures::join!(
        api.performance_curve(hero_id, Priority::Background),
        api.item_stats(hero_id, Priority::Background),
        api.ability_orders(hero_id, None, Priority::Background),
    );

    let orders = settle(orders, "ability orders", hero_id);

    HeroAverages {
        curve: settle(curve, "performance curve", hero_id),
        items: settle(items, "item stats", hero_id),
        most_played_order: analysis::ability_picks(&orders)
            .map(|picks| picks.most_played.abilities.clone()),
    }
}

pub async fn poll(
    api: &DeadlockApi,
    pool: &PgPool,
    now: Timestamp,
) -> Result<(PollOutcome, Vec<SummaryJob>)> {
    players::sync(pool).await?;
    let polled = players::all(pool).await?;
    if polled.is_empty() {
        return Ok((PollOutcome::default(), Vec::new()));
    }

    let account_ids: Vec<u32> =
        polled.iter().map(|player| player.account_id).collect();
    let polled_ids: Vec<i64> = account_ids.iter().copied().map(i64::from).collect();
    let since = now.checked_sub(LOOKBACK).unwrap_or(Timestamp::MIN);

    let mut fetched = api.recent_matches(&account_ids, since).await?;
    fetched.sort_by_key(|meta| meta.start_time);

    let mut outcome =
        PollOutcome { players: polled.len(), matches: fetched.len(), new: 0 };

    if fetched.is_empty() {
        players::mark_polled(pool, &polled_ids, now).await?;
        return Ok((outcome, Vec::new()));
    }

    let items = api.items(None, Priority::Background).await?;
    let index = ItemIndex::new(&items.value);

    let mut by_account: HashMap<u32, PolledPlayer> =
        polled.into_iter().map(|player| (player.account_id, player)).collect();
    let mut changed: HashSet<u32> = HashSet::new();
    let mut fresh: Vec<(i64, &MatchMetadata, &MatchPlayer)> = Vec::new();

    for meta in &fetched {
        for player in &meta.players {
            let Some(tracked) = by_account.get_mut(&player.account_id) else {
                continue;
            };
            if meta.start_time < tracked.seeded_at {
                continue;
            }

            let won = player.won(&meta.winning_team);
            let seen = SeenMatch {
                account_id: player.account_id,
                match_id: meta.match_id,
                hero_id: player.hero_id,
                won,
                start_time: meta.start_time,
            };
            if !matches::insert_if_new(pool, &seen).await? {
                continue;
            }

            tracked.streak = next_streak(tracked.streak, won);
            changed.insert(player.account_id);
            fresh.push((tracked.discord_id, meta, player));
            outcome.new += 1;
        }
    }

    for tracked in by_account.values().filter(|p| changed.contains(&p.account_id)) {
        players::set_streak(pool, tracked.account_id, tracked.streak).await?;
    }

    let hero_ids: BTreeSet<u32> =
        fresh.iter().map(|(_, _, player)| player.hero_id).collect();
    let mut averages: HashMap<u32, HeroAverages> = HashMap::new();
    for hero_id in hero_ids {
        averages.insert(hero_id, hero_averages(api, hero_id).await);
    }

    let empty = HeroAverages::default();
    let jobs = fresh
        .into_iter()
        .map(|(discord_id, meta, player)| SummaryJob {
            discord_id,
            summary: summary::summarize(
                meta,
                player,
                averages.get(&player.hero_id).unwrap_or(&empty),
                &index,
            ),
        })
        .collect();

    players::mark_polled(pool, &polled_ids, now).await?;

    Ok((outcome, jobs))
}
