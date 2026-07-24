# Audit: llamad2

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

A grab-bag of server-specific novelty handlers (~588 LOC): hello, goodmorning,
socials, dungeon/raid reports, counting-fail and "goof" counters. The notable
issues are two persistent counters stored as **flat JSON files written with
blocking `std::fs` on the async message path** — both a #3 (async) and #5 (data
placement) problem.

## Findings

### 1. Blocking `std::fs` counter persistence on async path  ·  #3  ·  med
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-24, folds in #2):** Both counters moved to the DB. New migration
  `0016_llamad2_counters` adds `llamad2_counters (name text PRIMARY KEY, count
  bigint NOT NULL DEFAULT 0)`; the two blocking read-modify-rewrite sites
  (`counting_fail.rs`, `goof.rs`) are replaced with a single **atomic** upsert —
  `INSERT … VALUES ($1, 1) ON CONFLICT (name) DO UPDATE SET count =
  llamad2_counters.count + 1 RETURNING count` (compile-time `query_scalar!`) — so
  the increment is server-side (`count = count + 1`, per
  [CC-9](_cross-cutting.md#cc-9), *not* read-then-absolute-write), removing both
  the reactor-blocking `std::fs` and the concurrent-message lost-update race in
  one change. This also fixes the pre-existing empty-file bug (a freshly
  `create`d file made `serde_json::from_str("")` error): a missing row now inserts
  count = 1. Both `run`s take `&PgPool` (`message_create` passes its `pool`; the
  `goof` binding passes `&cx.app.db`) and return `crate::Result` (new
  `LlamaD2Error::Database(#[from] sqlx::Error)`). Dropped the now-unused `serde`
  /`serde_json` deps, added `sqlx` (`cargo machete` clean). `.sqlx/` regenerated
  `--all-features` (one new entry; no LEFT JOIN, so plan-insensitive — the other
  cache entries are unchanged). **No regression test:** the fix is a DB-only
  atomic write with no pure-logic surface, and the workspace has no live-`PgPool`
  test harness ([CC-6](_cross-cutting.md#cc-6)) — same posture as gold-star/lfg
  DS-1, temp-voice DS-2; the compile-time SQL check is the net. Counters remain
  global (name-keyed), matching prior behaviour — no per-guild scope change.
- **Where:** `src/counting_fail.rs:31-48` (`OpenOptions`… `write_all`, file
  `countingFails.json`), `src/goof.rs:26-43` (file `dumbCount.json`). Both inside
  `async fn run(...)`.
- **What:** Each invocation opens, reads, and rewrites a JSON file synchronously
  on the async reactor.
- **Why it matters:** Blocking file I/O on an async task stalls the runtime
  thread; and a flat file in the process CWD is fragile (lost on redeploy, not
  shared across instances, races between concurrent messages).
- **Suggested fix:** Move both counters to a DB table (a single `counters` row
  per kind), read/write via `query!`. Removes the blocking I/O and the data-
  placement problem at once. See also [CC-5](_cross-cutting.md#cc-5).

### 2. Counter state belongs in the DB  ·  #5  ·  med
- **Status:** `in-review` (folded into finding #1)            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** same as #1 (`countingFails.json`, `dumbCount.json`).
- **What / Why / Fix:** Persistent per-guild counters stored outside the DB.
  Fold into the fix for #1. **Resolved by finding #1's fix** — both counters now
  live in the `llamad2_counters` DB table (migration `0016_llamad2_counters`).

### 3. No integration tests  ·  #6  ·  low
- **Where:** no `tests/` directory. Low priority — mostly cosmetic handlers.

## Clean
- #1 Architecture: one file per handler; simple.
- #4 Stringly typing: handler dispatch is in `bot/` bindings; nothing egregious.
- #7 Lint: no `#[expect]`/`#[allow]` in this crate.
