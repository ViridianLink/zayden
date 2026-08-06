//! Regression net for the cron scheduler's job-selection logic.
//!
//! Covers [bot.md #4](../../../design-docs/audits/bot.md) — the
//! `TODO(M9-correctness)` on the old `pending_jobs` retain predicate. The
//! predicate was not redundant: `includes(t)` genuinely rejects candidates,
//! because `jiff_cron`'s `Schedule::after` fast path skips the day-of-week
//! check. In the old code a rejection **deleted the job from the registry**;
//! `earliest_pending` takes `&[CronJob]` and merely skips it for that tick.
//!
//! **Mutation coverage** (the guard removed in turn, suite re-run, then
//! reverted — the fails-before evidence, since the logic was extracted to
//! `zayden_core` in the same task that fixed it):
//!
//! | Mutation of `next_run` | Result |
//! |---|---|
//! | `.find(includes)` → `.next()` (no guard) | caught — resolves to Thursday 17:00 |
//! | `.find(includes)` → `.next().filter(includes)` (the old predicate) | caught — job not scheduled at all |

use std::str::FromStr;

use jiff::civil::date;
use jiff::tz::TimeZone;
use jiff_cron::Schedule;
use zayden_core::{CronJob, earliest_pending, prune_exhausted};

/// `"0 0 17 * * Fri *"` — gambling's `lotto` / `higherlower` schedule.
const FRIDAY_1700: &str = "0 0 17 * * Fri *";
/// `"0 0 0 * * Mon *"` — destiny2's `endgame_analysis_sheet_weekly`.
const MONDAY_MIDNIGHT: &str = "0 0 0 * * Mon *";
/// `"0 */10 * * * * *"` — gambling's `stamina`.
const EVERY_10_MIN: &str = "0 */10 * * * * *";
/// A fired one-shot: the year-pinned shape `lfg::cron::create_reminders` builds,
/// with a year in the past.
const FIRED_ONE_SHOT: &str = "0 30 14 3 6 * 2024";
/// The same shape with a year still to come.
const FUTURE_ONE_SHOT: &str = "0 30 14 3 6 * 2027";

/// A `CronJob` from a schedule literal. Written as macros rather than helper fns
/// because `clippy.toml`'s `allow-unwrap-in-tests` only covers code inside a
/// `#[test]` item, and a free fn in a test binary is not one (gold-star
/// precedent).
macro_rules! job {
    ($id:expr, $source:expr) => {
        CronJob::new($id, $source).unwrap()
    };
}

/// A UTC `Zoned` from civil date/time parts.
macro_rules! utc {
    ($y:expr, $m:expr, $d:expr, $h:expr, $min:expr, $s:expr) => {
        utc!($y, $m, $d, $h, $min, $s, 0)
    };
    ($y:expr, $m:expr, $d:expr, $h:expr, $min:expr, $s:expr, $nanos:expr) => {
        date($y, $m, $d).at($h, $min, $s, $nanos).to_zoned(TimeZone::UTC).unwrap()
    };
}

/// The first pending run time, or a test failure.
macro_rules! first_run {
    ($pending:expr) => {
        $pending.first().expect("a job is pending").0.clone()
    };
}

/// The upstream defect the `includes` guard exists for.
///
/// `Schedule::after(t).next()` rounds `t` up to the next whole second and
/// returns it when seconds/minutes/hours/day-of-month/month/year all match —
/// without checking day-of-week. Sampled 500 ms before 17:00 on a **Thursday**,
/// a `Fri`-only schedule therefore proposes *Thursday* 17:00.
///
/// This test documents the upstream behaviour rather than the workspace's. If
/// `jiff_cron` ever fixes it, this test fails and the guard in `next_run` (and
/// this test) can be reconsidered — it must not be dropped silently.
#[test]
fn jiff_cron_after_can_propose_a_wrong_weekday() {
    let schedule = Schedule::from_str(FRIDAY_1700).unwrap();
    let thursday = utc!(2026, 8, 6, 16, 59, 59, 500_000_000);

    let proposed = schedule.after(thursday).next().expect("a candidate");

    assert_eq!(proposed, utc!(2026, 8, 6, 17, 0, 0));
    assert!(
        !schedule.includes(proposed),
        "the schedule itself rejects the time its own iterator proposed"
    );
}

/// The guard: a weekday-restricted job must be scheduled for its real next
/// occurrence, never for the wrong-weekday candidate above.
#[test]
fn weekday_restricted_job_resolves_past_a_wrong_weekday_candidate() {
    let jobs = [job!("lotto", FRIDAY_1700)];
    let thursday = utc!(2026, 8, 6, 16, 59, 59, 500_000_000);

    let pending = earliest_pending(&jobs, &thursday);

    assert_eq!(pending.len(), 1, "the job must still be scheduled");
    assert_eq!(
        first_run!(pending),
        utc!(2026, 8, 7, 17, 0, 0),
        "must resolve to Friday, not the Thursday candidate"
    );
}

/// The same window, but on a day the schedule *does* match: the fast-path
/// candidate is legitimate and must be taken as-is.
#[test]
fn weekday_restricted_job_takes_a_matching_same_day_candidate() {
    let jobs = [job!("lotto", FRIDAY_1700)];
    let friday = utc!(2026, 8, 7, 16, 59, 59, 500_000_000);

    let pending = earliest_pending(&jobs, &friday);

    assert_eq!(first_run!(pending), utc!(2026, 8, 7, 17, 0, 0));
}

/// Selection semantics preserved from the original `pending_jobs`: only the
/// strictly-earliest run time is returned.
#[test]
fn only_the_earliest_job_is_pending() {
    let jobs = [
        job!("lotto", FRIDAY_1700),
        job!("stamina", EVERY_10_MIN),
        job!("endgame", MONDAY_MIDNIGHT),
    ];
    let thursday = utc!(2026, 8, 6, 12, 0, 0);

    let pending = earliest_pending(&jobs, &thursday);

    assert_eq!(pending.len(), 1, "stamina alone is next");
    assert_eq!(first_run!(pending), utc!(2026, 8, 6, 12, 10, 0));
}

/// Selection semantics preserved: jobs tied at the same run time all fire.
#[test]
fn jobs_tied_at_the_earliest_run_time_all_fire() {
    let jobs = [
        job!("stamina", EVERY_10_MIN),
        job!("lotto", FRIDAY_1700),
        job!("higherlower", FRIDAY_1700),
    ];
    // 16:59:00 on a Friday: stamina and both Friday jobs are all due 17:00:00.
    let friday = utc!(2026, 8, 7, 16, 59, 0);

    let pending = earliest_pending(&jobs, &friday);

    assert_eq!(pending.len(), 3, "all three are due at 17:00");
    assert!(pending.iter().all(|(t, _)| *t == utc!(2026, 8, 7, 17, 0, 0)));
}

/// The run time is always strictly in the future, so the old `t > now` half of
/// the retain predicate was genuinely redundant — unlike `includes`.
#[test]
fn run_time_is_always_strictly_after_now() {
    let jobs = [job!("stamina", EVERY_10_MIN)];
    // Exactly on a firing boundary: the next run is the *following* slot.
    let now = utc!(2026, 8, 6, 12, 10, 0);

    let pending = earliest_pending(&jobs, &now);

    assert!(first_run!(pending) > now);
    assert_eq!(first_run!(pending), utc!(2026, 8, 6, 12, 20, 0));
}

/// An empty registry is not an error, and no job is invented.
#[test]
fn no_jobs_means_nothing_pending() {
    let pending = earliest_pending(&[], &utc!(2026, 8, 6, 12, 0, 0));

    assert!(pending.is_empty());
}

/// A fired one-shot is never selected — its schedule has no occurrence left.
#[test]
fn a_fired_one_shot_is_not_pending() {
    let jobs = [job!("lfg_1", FIRED_ONE_SHOT)];

    let pending = earliest_pending(&jobs, &utc!(2026, 8, 6, 12, 0, 0));

    assert!(pending.is_empty());
}

/// …and it is collected, so it stops costing a failed ordinal search per tick.
/// This is the LFG reminder lifecycle: `create_reminders` adds four year-pinned
/// jobs per post and only ever evicts that post's *own* id, so `prune_exhausted`
/// is the sole path by which a fired reminder leaves the registry.
#[test]
fn fired_one_shots_are_pruned() {
    let mut jobs = vec![
        job!("lfg_1", FIRED_ONE_SHOT),
        job!("lfg_1", FIRED_ONE_SHOT),
        job!("stamina", EVERY_10_MIN),
        job!("lfg_2", FUTURE_ONE_SHOT),
    ];

    prune_exhausted(&mut jobs, &utc!(2026, 8, 6, 12, 0, 0));

    let ids = jobs.iter().map(|job| job.id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, ["stamina", "lfg_2"], "only fired one-shots are dropped");
}

/// The pruning predicate must be "no occurrence left", **not** `next_run`'s
/// `includes` check. A `Fri`-only job sampled in the wrong-weekday window has a
/// candidate rejected — and must still survive, because it recurs forever.
/// This is the exact conflation that made the old `retain` delete live jobs.
#[test]
fn a_recurring_job_survives_the_wrong_weekday_window() {
    let mut jobs = vec![
        job!("lotto", FRIDAY_1700),
        job!("endgame", MONDAY_MIDNIGHT),
        job!("announce", "0 0 17,18 * * Sun,Thu *"),
    ];

    prune_exhausted(&mut jobs, &utc!(2026, 8, 6, 16, 59, 59, 500_000_000));

    assert_eq!(jobs.len(), 3, "no weekday-restricted job may be pruned");
}
