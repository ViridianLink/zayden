# Audit: music

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

One of the healthiest crates. Async concurrency is handled deliberately
(`tokio::sync::Mutex` for the per-guild `GuildPlayer`, `DashMap` for the players
map), settings are read through the M1 `SettingsRegistry` (`cx.app.settings.music`)
rather than ad-hoc, and it has the second-best test coverage in the workspace
(7 integration files: embeds, permissions, player, queue, resolve, spotify,
youtube). No CC-1 (in-memory manager, not DB-generic).

## Findings

### 4. `resolver_timeouts.rs` combines a paused clock with real localhost TCP → the suite is red on `main`  ·  #6  ·  med  ·  confirmed
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Recorded 2026-08-09** by the [CC-12](_cross-cutting.md) task, whose `cargo
  test` gate hit it. **Not fixed there** — one finding per task, and that task
  touched no Rust at all. Recorded rather than folded in, per the
  [honeypot #7](honeypot.md) precedent.
- **Where:** `bot-modules/music/tests/resolver_timeouts.rs:123`
  (`stream_client_does_not_cap_a_slow_but_healthy_stream`), and `:98`
  (`stream_client_gives_up_on_a_silent_upstream`) shares the hazard.
- **What:** Both tests are `#[tokio::test(start_paused = true)]` but drive a
  **real** `reqwest` client against a **real** loopback mock server. Under a
  paused clock tokio auto-advances virtual time whenever the runtime goes idle —
  which is exactly what happens while the OS completes the TCP handshake. So
  `stream_client()`'s `connect_timeout(HTTP_CONNECT_TIMEOUT)`
  (`zayden-app/src/services/http.rs:4`, 10 s) can elapse in virtual time before
  a loopback connect finishes in real time.
- **Failure scenario (reproduced):** `cargo test -p music --test
  resolver_timeouts` → **1 failed, 4 passed**, deterministically on this host,
  in 0.74 s:
  *"a healthy slow stream must not be cut off by the client: reqwest::Error {
  kind: Request, … ConnectError("tcp connect error", 127.0.0.1:59222,
  Custom { kind: TimedOut, error: Elapsed(()) }) }"*. Note the assertion never
  got to measure what it is about — it failed at **connect**, not at the
  per-read budget the test exists to pin.
- **Why it matters:** this is the workspace's mandated `cargo test` gate, and it
  is currently red on `main` for a reason unrelated to any change under review.
  A gate that fails for ambient reasons trains everyone to read red as noise —
  the same erosion [CC-12](_cross-cutting.md) is about, from the other direction.
  It also means the test is **not currently proving** the property its own
  doc-comment says it guards (that a per-read budget is not a total-request
  budget), which was [music #2](#)'s central distinction.
- **When it started:** the test arrived with `33281cd4` (music #2, 2026-08-05).
  The last recorded full-workspace green was `cf132327`'s *674 passed / 0
  failed*; the two tasks after it (`f6ccd11b`, `95ad9efc`) ran only
  `-p honeypot` plus a `--no-run` workspace compile, so this had no gate
  watching it in between. **Lesson worth keeping: a `--no-run` compile is not a
  test run**, and two consecutive tasks substituting one for the other is how a
  red suite stays unnoticed.
- **Suggested fix:** do not paper over it with a longer `connect_timeout`.
  Either (a) drop `start_paused` on the two tests that do real socket I/O and
  drive them on the real clock with small real durations, or (b) keep the paused
  clock and remove the real socket — assert the budgets against a
  `tokio::io::duplex` / in-memory transport so no connect is involved. (b) keeps
  the tests fast and deterministic and is the better end state; (a) is the
  low-churn path. Confirm the same hazard at `:98` either way.
- **Fix (2026-08-09).** Took **(a)**, because **(b) is not reachable**: `reqwest`
  exposes no public custom-connector API, so a `songbird_reqwest::Client` cannot
  be driven over `tokio::io::duplex` without replacing the very client under
  test. Taking (a) without papering over the connect budget needed one seam —
  `resolve/http.rs` grew `stream_client_with(connect, read)`, which
  `stream_client()` now calls with the production constants. The two socket
  tests drop `start_paused`, run on the real clock, and build the real client at
  millisecond budgets (5 s connect / 300 ms read). Whole file: 0.74 s, stable
  over 5 consecutive runs.
- **The `:98` hazard was real, and it was the more interesting half.**
  `stream_client_gives_up_on_a_silent_upstream` was *passing* — but only
  asserted `err.is_timeout()`, which is equally true of a **connect** timeout.
  So on this host it had quietly stopped exercising the read budget and would
  have passed with the read budget deleted. It now also asserts `!err.is_connect()`
  and that it gave up inside the connect budget, which is what makes it a test of
  the property its name claims. **Lesson: a green test sharing a red test's
  harness defect is not evidence the harness is fine** — the failing assertion is
  just the one whose predicate happened to be narrow enough to notice.
- **Rewriting the *scale* of the dribble test changed what it can prove, for the
  better.** The old version compared its 10 s gap against `STREAM_READ_TIMEOUT`
  and its 50 s total against a *foreign* constant (`HTTP_TIMEOUT`, 30 s) named
  only in a doc-comment — so the "a total-request cap would fail this" claim was
  asserted nowhere. The relationship, not the magnitude, is the property, so it
  now pins both sides at test scale: `GAP < read budget` **and**
  `GAP * BODY_LEN > read budget`. A total-request cap of the same size now fails
  the test by construction rather than by coincidence.
- **Verified by mutation** (both caught, on the fixed tree):
  `.read_timeout(read)` → `.timeout(read)` (per-read budget becomes a
  total-request one) fails the dribble test; removing the read budget entirely
  fails the silent-upstream test after 8 s. Plus the original fails-before,
  reproduced at task start: `1 failed, 4 passed` at `:143` on a `ConnectError`.
- **Also removed:** `tokio`'s `test-util` dev-feature in
  `bot-modules/music/Cargo.toml`, which existed only for the `start_paused`
  attributes this fix deletes and is now unused by the crate. `cargo machete`
  clean.
- **Residual:** `stream_client_with` is `pub` solely so an integration test can
  reach it — the project has no `pub(crate)`-plus-`#[cfg(test)]` seam available
  here, since the convention is `tests/` integration files (which see only the
  public API). Its doc-comment says production code must call `stream_client()`.
  The alternative — leaving the budgets unparameterised and running the tests at
  the real 10 s/20 s constants — costs ~20 s of wall time per `cargo test` on the
  workspace's mandated gate, which is the trade this rejects.

_No other findings outstanding._

## Clean
- #1 Architecture: clear `commands/` · `components/` · `resolve/` · manager /
  player / queue / voice split; settings via `SettingsRegistry`.
- #1 DB access: n/a — playback state is in-memory by design; no ad-hoc SQL.
- #3 Async: **correct** — `tokio::sync::Mutex` held across `.await` is
  intentional and safe; `DashMap` entries not held across `.await`.
- #4 Stringly typing: control-panel routing is namespaced; no raw domain strings.
- #6 Tests: 7 integration files covering real behaviour (queue ops, permission
  gating, resolver parsing).
