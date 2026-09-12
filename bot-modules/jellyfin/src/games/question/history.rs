use std::sync::Arc;

use sqlx::PgPool;

use crate::error::{JellyfinError, Result};
use crate::index::LibraryItemRow;
use crate::runtime::JellyfinRuntime;
use crate::transport::playback::model::RecentPlay;

const HISTORY_LIMIT: i64 = 40;
const CHOICES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub prompt: String,
    pub answer: String,
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    DeepDive,
    Recent,
    Yearly,
}

impl Tier {
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "deep-dive" => Self::DeepDive,
            "yearly" => Self::Yearly,
            _ => Self::Recent,
        }
    }
}

pub async fn own_history(
    runtime: &Arc<JellyfinRuntime>,
    jellyfin_user_id: &str,
) -> Result<Vec<RecentPlay>> {
    if let Some(cached) = runtime.caches.recent_plays.get(jellyfin_user_id).await {
        return Ok(cached);
    }

    let plays = runtime
        .playback
        .recent_plays(jellyfin_user_id, HISTORY_LIMIT)
        .await
        .map_err(JellyfinError::from)?;

    runtime
        .caches
        .recent_plays
        .insert(jellyfin_user_id.to_owned(), plays.clone())
        .await;

    Ok(plays)
}

pub async fn server_history(
    runtime: &Arc<JellyfinRuntime>,
) -> Result<Vec<RecentPlay>> {
    runtime
        .playback
        .server_recent_plays(HISTORY_LIMIT)
        .await
        .map_err(JellyfinError::from)
}

pub async fn build(
    pool: &PgPool,
    plays: &[RecentPlay],
    tier: Tier,
) -> Result<Option<Question>> {
    let window = match tier {
        Tier::DeepDive => 1,
        Tier::Recent => 10,
        Tier::Yearly => plays.len(),
    };

    let considered: Vec<&RecentPlay> = plays.iter().take(window).collect();
    let Some(target) = considered.first() else {
        return Ok(None);
    };

    let (prompt, answer) = match tier {
        Tier::DeepDive => {
            ("What was the last thing watched?".to_owned(), target.item_name.clone())
        },
        Tier::Recent => {
            let longest = longest_of(pool, &considered).await?;
            match longest {
                Some(name) => (
                    "Of the last 10 titles watched, which has the longest runtime?"
                        .to_owned(),
                    name,
                ),
                None => return Ok(None),
            }
        },
        Tier::Yearly => {
            let most = most_watched(&considered);
            match most {
                Some(name) => (
                    "Which title comes up most often in this watch history?"
                        .to_owned(),
                    name,
                ),
                None => return Ok(None),
            }
        },
    };

    let mut choices = vec![answer.clone()];
    for row in LibraryItemRow::search(pool, "", Some("Movie"), 60).await? {
        if choices.len() >= CHOICES {
            break;
        }
        if !choices.contains(&row.name) {
            choices.push(row.name);
        }
    }

    if choices.len() < 2 {
        return Ok(None);
    }

    // A stable but content-derived rotation, so the answer is not always first
    // without pulling in a random number generator for one call site.
    let shift = answer.len() % choices.len();
    choices.rotate_left(shift);

    Ok(Some(Question { prompt, answer, choices }))
}

async fn longest_of(pool: &PgPool, plays: &[&RecentPlay]) -> Result<Option<String>> {
    let mut best: Option<(i64, String)> = None;

    for play in plays {
        let Some(row) = LibraryItemRow::by_id(pool, &play.item_id).await? else {
            continue;
        };
        let ticks = row.runtime_ticks.unwrap_or(0);

        if best.as_ref().is_none_or(|(current, _)| ticks > *current) {
            best = Some((ticks, row.name));
        }
    }

    Ok(best.map(|(_, name)| name))
}

fn most_watched(plays: &[&RecentPlay]) -> Option<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();

    for play in plays {
        match counts.iter_mut().find(|(name, _)| *name == play.item_name) {
            Some((_, count)) => *count += 1,
            None => counts.push((play.item_name.clone(), 1)),
        }
    }

    counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    counts.into_iter().next().map(|(name, _)| name)
}
