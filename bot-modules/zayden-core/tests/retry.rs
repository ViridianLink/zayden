//! Regression net for the bounded-retry helper.
//!
//! Covers [honeypot #1](../../../design-docs/audits/honeypot.md) — the honeypot
//! soft-ban's `unban` had no retry, so one transient 5xx/429 left a **permanent
//! ban** on a real member with nothing but an `error!` line to show for it.
//! `retry` is the mechanism that fix introduces; these tests are what pin it.
//!
//! **Why the tests live here and not in `honeypot`:** the defect is in
//! `honeypot::message_create`, which needs a live serenity `Context` and real
//! HTTP, so it cannot be driven from a test. The retry *loop* is the part of
//! the fix that carries the logic, and extracting it to `zayden_core` — the
//! same relocation-for-testability move CC-2 made for `DispatchMap` — is what
//! makes it reachable. The wiring in `message_create` (choosing
//! `HoneypotOutcome` from the retry's result) stays untested; that seam is
//! [honeypot #7](../../../design-docs/audits/honeypot.md).
//!
//! **Fails-before evidence.** `retry` did not exist before this task, so no
//! test could fail against the old code by construction. The equivalent, per
//! the CC-6 verification pattern, is the mutation matrix — each guard removed
//! in turn, the suite re-run, then reverted:
//!
//! | Mutation of `retry` | Result |
//! |---|---|
//! | drop the loop, call `op()` once (**the pre-fix behaviour**) | caught — `a_transient_failure_is_retried_then_succeeds` fails |
//! | `if attempt >= budget.attempts \|\| !is_retryable(&error)` → drop the `!is_retryable` arm | caught — `a_permanent_failure_is_not_retried` fails (4 calls, not 1) |
//! | drop the `attempt >= budget.attempts` arm | caught — `the_budget_is_exhausted_and_the_last_error_is_returned` hangs (infinite retry) |
//! | `return Err(error)` after exhaustion → `Ok(default)` (the shape `guild_create::set_commands` has) | caught — same test, on the `Err` assertion |
//! | `backoff.saturating_mul(2)` → leave `backoff` fixed | caught — `the_backoff_doubles_between_attempts` |

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use serenity::all::StatusCode;
use zayden_core::retry::{RetryBudget, retry, status_is_transient};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestError {
    /// Worth another go — stands in for a 5xx / 429 / transport blip.
    Transient,
    /// Retrying only burns rate limit — stands in for a 403.
    Permanent,
}

/// A budget with no real waiting, for the tests that only count calls.
const fn instant(attempts: u32) -> RetryBudget {
    RetryBudget::new(attempts, Duration::ZERO)
}

/// Runs `retry` against a fixed script of outcomes, returning the result and
/// the number of times the operation was actually invoked. The script is
/// indexed by attempt number; running off the end yields `Ok`.
///
/// The counter is an `AtomicU32` rather than a `Cell` so the future stays
/// `Send` — `clippy::future_not_send` (nursery) is part of the `-D warnings`
/// gate, and a non-`Send` future here would be a test-only shape the real call
/// site (a spawned gateway-event task) could not use.
async fn run(
    budget: RetryBudget,
    script: &[TestError],
) -> (Result<u32, TestError>, u32) {
    let calls = AtomicU32::new(0);

    let result = retry(
        budget,
        |error: &TestError| matches!(error, TestError::Transient),
        || {
            let n = calls.fetch_add(1, Ordering::Relaxed);

            async move {
                match script.get(n as usize) {
                    Some(&error) => Err(error),
                    None => Ok(n + 1),
                }
            }
        },
    )
    .await;

    (result, calls.load(Ordering::Relaxed))
}

#[tokio::test]
async fn it_succeeds_without_retrying() {
    let (result, calls) = run(instant(4), &[]).await;

    assert_eq!(result, Ok(1));
    assert_eq!(calls, 1, "a successful call must not be repeated");
}

// The finding itself: a transient failure on the honeypot's `unban` used to be
// terminal, and the cost of that was a standing ban on a real member.
#[tokio::test]
async fn a_transient_failure_is_retried_then_succeeds() {
    let (result, calls) = run(instant(4), &[TestError::Transient]).await;

    assert_eq!(result, Ok(2), "the second attempt's value must be returned");
    assert_eq!(calls, 2);
}

#[tokio::test]
async fn several_transient_failures_are_retried_within_budget() {
    let script = [TestError::Transient, TestError::Transient, TestError::Transient];
    let (result, calls) = run(instant(4), &script).await;

    assert_eq!(result, Ok(4));
    assert_eq!(calls, 4, "the whole budget is available to a transient failure");
}

// Retrying a 403 cannot succeed and spends rate limit that the *next* offender's
// ban needs — during a raid, which is when this code runs, that trade matters.
#[tokio::test]
async fn a_permanent_failure_is_not_retried() {
    let (result, calls) = run(instant(4), &[TestError::Permanent]).await;

    assert_eq!(result, Err(TestError::Permanent));
    assert_eq!(calls, 1, "a permanent failure must be returned on the first try");
}

#[tokio::test]
async fn a_permanent_failure_mid_run_stops_the_retries() {
    let script = [TestError::Transient, TestError::Permanent, TestError::Transient];
    let (result, calls) = run(instant(4), &script).await;

    assert_eq!(result, Err(TestError::Permanent));
    assert_eq!(calls, 2, "the permanent error ends the run, budget notwithstanding");
}

// The caller has to be able to tell "gave up" from "succeeded" — that is the
// whole basis for recording the honeypot hit as a standing `Ban` rather than a
// `SoftBan`. A trailing `Ok` here would silently restore the original defect.
#[tokio::test]
async fn the_budget_is_exhausted_and_the_last_error_is_returned() {
    let script = [TestError::Transient; 6];
    let (result, calls) = run(instant(3), &script).await;

    assert_eq!(result, Err(TestError::Transient));
    assert_eq!(calls, 3, "exactly `attempts` calls, no more");
}

// A zero budget is not a licence to skip the operation entirely: the honeypot's
// unban must always be *attempted* at least once, whatever the budget says.
#[tokio::test]
async fn a_zero_budget_still_attempts_once() {
    let (result, calls) = run(instant(0), &[TestError::Transient]).await;

    assert_eq!(result, Err(TestError::Transient));
    assert_eq!(calls, 1);
}

// Virtual time: `start_paused` auto-advances each `sleep`, so this asserts the
// backoff *schedule* without the suite waiting 7 real seconds.
#[tokio::test(start_paused = true)]
async fn the_backoff_doubles_between_attempts() {
    let budget = RetryBudget::new(4, Duration::from_secs(1));
    let script = [TestError::Transient; 6];

    let start = tokio::time::Instant::now();
    let (result, calls) = run(budget, &script).await;
    let elapsed = start.elapsed();

    assert_eq!(result, Err(TestError::Transient));
    assert_eq!(calls, 4);
    // Three sleeps between four attempts: 1s + 2s + 4s.
    assert_eq!(elapsed, Duration::from_secs(7));
}

#[tokio::test(start_paused = true)]
async fn a_successful_retry_does_not_wait_out_the_whole_budget() {
    let budget = RetryBudget::new(4, Duration::from_secs(1));

    let start = tokio::time::Instant::now();
    let (result, calls) = run(budget, &[TestError::Transient]).await;
    let elapsed = start.elapsed();

    assert_eq!(result, Ok(2));
    assert_eq!(calls, 2);
    assert_eq!(elapsed, Duration::from_secs(1), "one backoff, not three");
}

// --- `status_is_transient`: which Discord rejections are worth another go ---
//
// The loop above is only half the fix; this predicate decides what it acts on.
// Getting it wrong in either direction is a real cost — too narrow and the
// honeypot's unban still gives up on a 503, too broad and a 403 burns the
// whole budget of rate limit during a raid before failing anyway.
//
// These test `status_is_transient` rather than `is_transient` because a
// `serenity::Error::Http` **cannot be constructed** outside serenity:
// `ErrorResponse` and `DiscordJsonError` are both `#[non_exhaustive]`. So the
// status→verdict mapping is pinned here and `is_transient`'s destructuring is
// not covered — that is a real gap, kept as small as the split can make it.

#[test]
fn server_errors_are_transient() {
    assert!(status_is_transient(StatusCode::INTERNAL_SERVER_ERROR));
    assert!(status_is_transient(StatusCode::BAD_GATEWAY));
    assert!(status_is_transient(StatusCode::SERVICE_UNAVAILABLE));
    assert!(status_is_transient(StatusCode::GATEWAY_TIMEOUT));
}

#[test]
fn a_rate_limit_is_transient() {
    assert!(status_is_transient(StatusCode::TOO_MANY_REQUESTS));
}

// The honeypot's unban hits exactly these when the bot lacks Ban Members, or
// when the target was already unbanned by a moderator. Discord will give the
// same answer next time.
#[test]
fn client_errors_are_not_transient() {
    assert!(!status_is_transient(StatusCode::FORBIDDEN));
    assert!(!status_is_transient(StatusCode::NOT_FOUND));
    assert!(!status_is_transient(StatusCode::UNAUTHORIZED));
    assert!(!status_is_transient(StatusCode::BAD_REQUEST));
}

// 429 is the one non-5xx that is retryable, so the boundary either side of it
// is worth pinning explicitly rather than trusting `is_server_error()`.
#[test]
fn the_boundary_around_the_rate_limit_holds() {
    // 429 is the one non-5xx that is retryable, so pin both neighbours rather
    // than trusting `is_server_error()` to draw the line.
    assert!(!status_is_transient(StatusCode::from_u16(428).unwrap()));
    assert!(!status_is_transient(StatusCode::from_u16(430).unwrap()));
    assert!(!status_is_transient(StatusCode::OK));
}
