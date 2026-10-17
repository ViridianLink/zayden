use deadlock_api::Priority;
use deadlock_api::model::PatchNote;
use jiff::{SignedDuration, Timestamp};
use sqlx::PgPool;
use tracing::{error, warn};

use crate::client::DeadlockApi;
use crate::db::patches::{self, NewPatch};
use crate::db::snapshots;
use crate::db::watch_state::{self, WatchKey};
use crate::diff::{self, PatchDiff, Snapshot};
use crate::error::{DeadlockError, Result};

pub const PATCH_SOURCE: &str = "forum";
pub const BUILD_GRACE: SignedDuration = SignedDuration::from_mins(30);
pub const MAX_NOTE_AGE: SignedDuration = SignedDuration::from_hours(48);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct WatchOutcome {
    pub seeded: bool,
    pub new_version: Option<u32>,
    pub new_notes: usize,
    pub diffed: usize,
}

pub async fn run(
    api: &DeadlockApi,
    pool: &PgPool,
    now: Timestamp,
) -> Result<WatchOutcome> {
    let mut outcome = WatchOutcome::default();

    let versions = api.client_versions(Priority::Background).await?;
    if versions.stale {
        return Ok(outcome);
    }
    let Some(latest) = versions.value.iter().copied().max() else {
        return Ok(outcome);
    };

    match watch_state::get(pool, WatchKey::Latest).await? {
        None => {
            outcome.seeded = seed(api, pool, latest, now).await?;
            return Ok(outcome);
        },
        Some(stored) if latest > stored => {
            if !capture(api, pool, latest).await? {
                return Ok(outcome);
            }
            watch_state::set(pool, WatchKey::Latest, latest).await?;
            api.invalidate_unversioned().await?;
            outcome.new_version = Some(latest);
        },
        Some(_) => {},
    }

    let notes = api.patch_notes(Priority::Background).await?;
    if notes.stale {
        return Ok(outcome);
    }
    outcome.new_notes = store_notes(pool, &notes.value, now, false).await?;

    outcome.diffed = settle_diffs(pool, now).await?;

    Ok(outcome)
}

async fn capture(api: &DeadlockApi, pool: &PgPool, version: u32) -> Result<bool> {
    let heroes = api.heroes(Some(version), Priority::Background).await?;
    let items = api.items(Some(version), Priority::Background).await?;
    if heroes.stale || items.stale {
        return Ok(false);
    }

    snapshots::store(pool, version, &heroes.value, &items.value).await?;

    Ok(true)
}

async fn seed(
    api: &DeadlockApi,
    pool: &PgPool,
    latest: u32,
    now: Timestamp,
) -> Result<bool> {
    if !capture(api, pool, latest).await? {
        return Ok(false);
    }

    let notes = api.patch_notes(Priority::Background).await?;
    if notes.stale {
        return Ok(false);
    }
    store_notes(pool, &notes.value, now, true).await?;

    watch_state::set(pool, WatchKey::Latest, latest).await?;
    watch_state::set(pool, WatchKey::Baseline, latest).await?;

    Ok(true)
}

fn parse_date(raw: &str) -> Result<Timestamp> {
    raw.parse().map_err(|_error| DeadlockError::PatchDate(raw.to_owned()))
}

async fn store_notes(
    pool: &PgPool,
    notes: &[PatchNote],
    now: Timestamp,
    seeding: bool,
) -> Result<usize> {
    let known = patches::known_guids(pool).await?;
    let mut fresh = 0;

    for note in notes {
        if note.source != PATCH_SOURCE || known.contains(&note.guid.text) {
            continue;
        }

        let published_at = match parse_date(&note.pub_date) {
            Ok(published_at) => published_at,
            Err(e) => {
                warn!(error = ?e, guid = note.guid.text, "deadlock: skipping a patch note");
                continue;
            },
        };

        let announced = seeding || now.duration_since(published_at) > MAX_NOTE_AGE;
        let patch = NewPatch {
            guid: &note.guid.text,
            title: &note.title,
            link: &note.link,
            published_at,
        };
        patches::insert(pool, &patch, announced).await?;

        if !announced {
            fresh += 1;
        }
    }

    Ok(fresh)
}

async fn patch_diff(
    pool: &PgPool,
    baseline: u32,
    current: u32,
) -> Result<PatchDiff> {
    let old = snapshots::load(pool, baseline).await?;
    let new = snapshots::load(pool, current).await?;

    let (Some((old_heroes, old_items)), Some((new_heroes, new_items))) = (old, new)
    else {
        error!(
            baseline,
            current, "deadlock: missing asset snapshot; diff left empty"
        );
        return Ok(PatchDiff::default());
    };

    Ok(diff::diff(&Snapshot { heroes: &old_heroes, items: &old_items }, &Snapshot {
        heroes: &new_heroes,
        items: &new_items,
    }))
}

async fn settle_diffs(pool: &PgPool, now: Timestamp) -> Result<usize> {
    let (Some(mut baseline), Some(current)) = (
        watch_state::get(pool, WatchKey::Baseline).await?,
        watch_state::get(pool, WatchKey::Latest).await?,
    ) else {
        return Ok(0);
    };

    let mut diffed = 0;

    for row in patches::awaiting_diff(pool).await? {
        if current == baseline && now.duration_since(row.detected_at) < BUILD_GRACE {
            break;
        }

        let diff = if current == baseline {
            PatchDiff::default()
        } else {
            patch_diff(pool, baseline, current).await?
        };

        patches::set_diff(pool, &row.guid, baseline, current, &diff).await?;
        watch_state::set(pool, WatchKey::Baseline, current).await?;
        baseline = current;
        diffed += 1;
    }

    Ok(diffed)
}
