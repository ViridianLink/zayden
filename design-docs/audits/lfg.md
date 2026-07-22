# Audit: lfg

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Well-structured (the `actions`/`components`/`commands`/`modals`/`models` split
that temp-voice and others mirror), but carries the DB-generic `async_trait`
pattern throughout (CC-1) and ships **zero** `tests/` despite ~3.3k LOC of post
lifecycle, slot/alternate bookkeeping, and reminder-cron logic. Structurally the
best migration reference alongside temp-voice.

## Findings

### 1. DB-generic `async_trait` managers  ·  #1  ·  high
- **Where:** `src/guild_manager.rs`, `src/models/{post,timezone_manager,mod}.rs`,
  all `commands/*`, `components/*`, `actions/*`, `modals/*`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1).

### 2. No integration tests  ·  #6  ·  high
- **Where:** no `tests/` directory.
- **What:** The post lifecycle (join/leave/alternate/kick, slot counting) and the
  reminder cron have no coverage — the highest-value untested logic in the
  workspace by LOC.
- **Why it matters:** Slot/alternate accounting bugs are easy to introduce and
  invisible without tests.
- **Suggested fix:** Add `tests/` for the pure post/slot state transitions
  first. See [CC-6](_cross-cutting.md#cc-6).

### 3. `#[expect]` escape-hatches  ·  #7  ·  low
- **Where:** `src/actions/leave.rs:19`, `src/cron/reminders.rs:20`.
- **What / Why / Fix:** See [CC-3](_cross-cutting.md#cc-3).

### 4. `setup` duplicates the dashboard; `tags` CRUD belongs on the web  ·  #8  ·  med
- **Where:** `src/commands/setup.rs` (writes `lfg_settings` via `Manager::insert`),
  `src/commands/tags.rs`.
- **What:** `setup` writes the exact `lfg_settings` row the dashboard already
  writes via `save_lfg_settings` — an active duplicate editor. `tags` is admin
  CRUD of reference data, a natural web form.
- **Why it matters:** Two write paths to one table diverge over time; a one-shot
  config command is a worse form than the settings page that already exists.
- **Suggested fix:** Make the dashboard the single editor; remove `setup` or
  reduce it to a deep-link. Add a tags page. Keep create/join/leave/kick (live
  post interaction) in-bot. See [CC-8](_cross-cutting.md#cc-8).

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lens: concurrency/atomicity._

### DS-1. Fireteam capacity race → post overfills past `fireteam_size`  ·  Pass 2  ·  med
- **Status:** `complete — 82f308a2`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-19):** `join` now takes a `SELECT id FROM lfg_posts WHERE id = $1
  FOR UPDATE` row lock (`sql/lfg/PostManager/lock_post.sql`) as the first
  statement of its transaction, before the fireteam `INSERT` and the aggregate
  re-read (`bot/src/bindings/lfg/mod.rs:100-102`). Same-tick joins on one post now
  serialise on that row lock: the second waiter blocks until the first commits,
  then its `post_row` re-read sees the peer's committed insert and the existing
  `fireteam_len() > fireteam_size()` guard correctly rejects the overfill.
- **Test note:** No fails-before/passes-after regression test was added — it is not
  feasible in-workspace. The fix is a pure transaction-serialisation change with no
  new pure-logic surface (the capacity predicate was already correct and is
  unchanged), so it can only be exercised by two concurrent live transactions. The
  `join` impl lives in the `bot` binary (no lib target — see CC-2), unreachable from
  an integration test, and the workspace has no DB test harness (CC-6). Verified
  instead by live compile-time SQL validation + workspace clippy/test gate.
- **Where:** `bot/src/bindings/lfg/mod.rs:89-125` (`join`), SQL
  `sql/lfg/PostManager/join.sql` + `post_row.sql`; command path
  `bot-modules/lfg/src/actions/join.rs:68`,
  `bot-modules/lfg/src/components/join.rs:18`.
- **What:** `join` opens a tx, `INSERT INTO lfg_fireteam …` (a *new row* per
  user), re-reads the aggregated `post_row`, and rejects with `FireteamFull` iff
  `fireteam_len() > fireteam_size()`. Because each join inserts a **distinct**
  `lfg_fireteam` row (different `user_id`), the two transactions do **not**
  contend on any shared row lock — unlike an `UPDATE` on the post row would. Under
  Postgres' default `READ COMMITTED`, each tx's re-read sees its own insert plus
  only *committed* peers, not the concurrent uncommitted insert.
- **Failure scenario:** a 6-slot post currently has 5 members. Users A and B click
  **Join** in the same tick. Tx A inserts A, re-reads → 6 members, `6 > 6` is
  false → commit. Tx B (concurrent) inserts B, re-reads → its snapshot still shows
  5 committed + B = 6, `6 > 6` false → commit. Final fireteam = **7 members**, one
  over `fireteam_size`. N simultaneous clicks against the same pre-image → `5 + N`
  members. The invariant "`fireteam` never exceeds `fireteam_size`" is broken and
  stays broken (later joins then see `>size` and are correctly rejected, so the
  post is stuck over capacity).
- **Suggested fix:** serialize the check by taking a row lock on the post first —
  `SELECT id FROM lfg_posts WHERE id = $1 FOR UPDATE` at the top of the join tx —
  or enforce capacity with a DB constraint/trigger on `lfg_fireteam` count.
  **Confidence: confirmed** (distinct-row inserts + `READ COMMITTED` snapshot;
  the check is `>` on a stale count).

## Clean
- #1 Architecture: clean `actions`/`components`/`commands`/`modals`/`models`
  layering; `ModuleComponent`/`ModuleModal` wired in `bot/`.
- #1 DB access: concrete impls use compile-time macros (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
