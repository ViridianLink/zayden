use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use crate::error::Result;
use crate::index::LibraryItemRow;

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub title: String,
    pub year: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub rating: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct GapReport {
    pub total: usize,
    pub matched_by_id: usize,
    pub matched_by_title: usize,
    pub unmatched: usize,
    pub missing: Vec<Candidate>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Key {
    Tmdb(i32),
    Title(String),
}

#[must_use]
pub fn collapse(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut seen: HashMap<Key, usize> = HashMap::with_capacity(candidates.len());
    let mut unique: Vec<Candidate> = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        let key = candidate
            .tmdb_id
            .map_or_else(|| Key::Title(candidate.title.to_lowercase()), Key::Tmdb);

        match seen.entry(key) {
            Entry::Occupied(slot) => {
                if let Some(kept) = unique.get_mut(*slot.get()) {
                    kept.rating = best(kept.rating, candidate.rating);
                }
            },
            Entry::Vacant(slot) => {
                slot.insert(unique.len());
                unique.push(candidate);
            },
        }
    }

    unique
}

fn best(a: Option<f32>, b: Option<f32>) -> Option<f32> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

pub async fn cross_reference(
    pool: &PgPool,
    item_type: &str,
    candidates: Vec<Candidate>,
    min_rating: f32,
    limit: usize,
) -> Result<GapReport> {
    let candidates = collapse(candidates);
    let owned: HashSet<i32> =
        LibraryItemRow::tmdb_ids(pool, item_type).await?.into_iter().collect();

    let mut report = GapReport { total: candidates.len(), ..Default::default() };
    let mut missing = Vec::new();

    for candidate in candidates {
        match candidate.tmdb_id {
            Some(tmdb_id) => {
                report.matched_by_id += 1;
                if !owned.contains(&tmdb_id) {
                    missing.push(candidate);
                }
            },
            None => {
                // No id in the diary: fall back to title, and say so in the
                // report rather than presenting it as an exact match.
                let found = LibraryItemRow::search(
                    pool,
                    &candidate.title,
                    Some(item_type),
                    1,
                )
                .await?;

                if found.is_empty() {
                    report.unmatched += 1;
                    missing.push(candidate);
                } else {
                    report.matched_by_title += 1;
                }
            },
        }
    }

    missing.retain(|c| c.rating.is_none_or(|r| r >= min_rating));
    missing
        .sort_by(|a, b| b.rating.unwrap_or(0.0).total_cmp(&a.rating.unwrap_or(0.0)));
    missing.truncate(limit);

    report.missing = missing;
    Ok(report)
}
