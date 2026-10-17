use std::collections::HashMap;

use deadlock_api::model::AccountRank;
use deadlock_api::rank::Badge;
use jiff::Timestamp;
use sqlx::PgPool;

use crate::client::DeadlockApi;
use crate::db::{players, rank_snapshots};
use crate::error::Result;
use crate::squad::{RankChange, next_announcement, rank_change};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquadEventKind {
    RankUp { from: Badge, to: Badge },
    RankDown { from: Badge, to: Badge },
    Streak(i32),
}

#[derive(Debug, Clone, Copy)]
pub struct SquadEvent {
    pub discord_id: i64,
    pub kind: SquadEventKind,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RankPollOutcome {
    pub players: usize,
    pub changed: usize,
    pub events: usize,
}

pub async fn poll(
    api: &DeadlockApi,
    pool: &PgPool,
    now: Timestamp,
) -> Result<(RankPollOutcome, Vec<SquadEvent>)> {
    players::sync(pool).await?;
    let tracked = players::squad_players(pool).await?;
    if tracked.is_empty() {
        return Ok((RankPollOutcome::default(), Vec::new()));
    }

    let account_ids: Vec<u32> =
        tracked.iter().map(|player| player.account_id).collect();
    let ranks: HashMap<u32, AccountRank> = api
        .player_ranks(&account_ids)
        .await?
        .into_iter()
        .map(|rank| (rank.account_id, rank))
        .collect();

    let mut outcome =
        RankPollOutcome { players: tracked.len(), ..RankPollOutcome::default() };
    let mut events = Vec::new();

    for player in &tracked {
        if let Some(rank) = ranks.get(&player.account_id) {
            let current = rank.badge();
            let change = rank_change(player.badge, current);

            if change != RankChange::Unchanged {
                let raw = i32::from(current.get());
                players::set_badge(pool, player.account_id, raw).await?;
                rank_snapshots::insert(pool, player.account_id, raw, now).await?;
            }

            match change {
                RankChange::Promoted { from, to } => {
                    outcome.changed += 1;
                    events.push(SquadEvent {
                        discord_id: player.discord_id,
                        kind: SquadEventKind::RankUp { from, to },
                    });
                },
                RankChange::Demoted { from, to } => {
                    outcome.changed += 1;
                    events.push(SquadEvent {
                        discord_id: player.discord_id,
                        kind: SquadEventKind::RankDown { from, to },
                    });
                },
                RankChange::Seeded | RankChange::Unchanged => {},
            }
        }

        let (announced, milestone) =
            next_announcement(player.announced_streak, player.streak);
        if announced != player.announced_streak {
            players::set_announced_streak(pool, player.account_id, announced)
                .await?;
        }
        if let Some(milestone) = milestone {
            events.push(SquadEvent {
                discord_id: player.discord_id,
                kind: SquadEventKind::Streak(milestone),
            });
        }
    }

    outcome.events = events.len();

    Ok((outcome, events))
}
