# Audit: family

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Clean `commands/` + `components/` + manager + `relationships.rs` structure for a
relationship-graph feature (~1.5k LOC). Carries the DB-generic `async_trait`
pattern (CC-1) and — notably — its *only* tests are **inline** `#[cfg(test)]`
modules (CC-2), which is both a convention violation and effectively no
`tests/`-visible coverage.

## Findings

### 1. DB-generic `async_trait` manager  ·  #1  ·  high
- **Where:** `src/family_manager.rs`, all `commands/*`, `components/*`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1).

### 2. Tests are inline `#[cfg(test)]` in `src/`  ·  #6  ·  med
- **Where:** `src/family_manager.rs:125`,
  `src/commands/information/siblings.rs:90`.
- **What:** The relationship-graph logic *is* tested, but inline in `src/` —
  violating the `tests/`-only convention (CC-2) and leaving the crate with no
  `tests/` directory.
- **Why it matters:** Convention drift, and the relationship-resolution logic
  (siblings/parents/children graph walks) is high-value coverage that should be
  visible in `tests/`.
- **Suggested fix:** Relocate both modules to `tests/`, exposing the minimum
  `pub(crate)` surface needed. See [CC-2](_cross-cutting.md#cc-2).

## Clean
- #1 Architecture: clean command/component/manager/relationships split.
- #1 DB access: concrete impl uses compile-time macros (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
- #7 Lint: one `#[expect]` at `commands/tree.rs:71` (CC-3).

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lenses: silent-failure, state-machine/invariant, concurrency._

### DS-1. `/block` is never enforced and `/unblock` never persists → entire block feature is inert  ·  Pass 1+7  ·  med
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot/src/bindings/family.rs:206-277` (`FamilyTable::save` — additive
  `INSERT … ON CONFLICT DO NOTHING`, no `DELETE` for blocks);
  `bot-modules/family/src/commands/block.rs:53-79` (`Unblock::run` →
  `remove_blocked` + `save`); `bot-modules/family/src/commands/adopt.rs:19-63`
  and `commands/marry.rs:21-66` (neither reads `blocked_ids`). `blocked_ids` is
  written by `add_blocked`/`remove_blocked` and loaded by `row`, but the grep
  (`blocked_ids` usages) shows **no read site** that gates a proposal.
- **What:** Two defects compound into a fully non-functional feature the UI claims
  works:
  1. **Block is a no-op.** `/block` inserts a `family_blocks` row and replies
     "User blocked.", but `adopt`/`marry` never consult `blocked_ids`, so a
     blocked user can still adopt/marry the blocker. The command's own
     description ("Blocks a user from being able to adopt/marry/etc you") is
     never honoured.
  2. **Unblock cannot remove the row.** `Unblock::run` mutates the in-memory
     `blocked_ids` vector via `remove_blocked`, then calls `save` — but `save`
     only issues `INSERT … ON CONFLICT DO NOTHING` per remaining id and has **no**
     `DELETE`. The removed block is never deleted from `family_blocks`; the
     command replies "User unblocked." while the DB is unchanged. (Contrast
     `remove_partner`, which is a dedicated `DELETE` — blocks have no equivalent.)
- **Failure scenario:** User A runs `/block @B` → `family_blocks (A,B)` inserted,
  reply "User blocked." B then runs `/marry @A` or `/adopt @A` → succeeds, because
  no code reads `blocked_ids`. A runs `/unblock @B` → reply "User unblocked." but
  `SELECT * FROM family_blocks WHERE user_id = A` still returns `(A,B)`. Every
  subsequent `/block`/`/unblock` of anyone re-persists the stale row (additive
  save re-inserts all `blocked_ids`).
- **Why it matters:** A user-facing safety feature (blocking unwanted
  adopt/marry proposals) is silently dead in both directions. Users relying on it
  are unprotected and cannot tell, because both commands report success.
- **Confidence:** confirmed (read all four paths + schema; the enforcement read
  site does not exist).
- **Suggested fix:** (a) Gate `adopt`/`marry` (and their accept handlers) on the
  target's `blocked_ids`. (b) Give blocks a dedicated `DELETE` path
  (`remove_block(pool, user, blocked)`) mirroring `remove_partner`, instead of
  routing removal through the additive `save`. Note the additive `save` is a
  general hazard: **any** future "remove a relationship by mutating the vec + save"
  will silently no-op the same way.

### DS-2. `marry`/`adopt` accept handlers re-run no invariant checks → `MAX_PARTNERS`/already-adopted bypass  ·  Pass 7  ·  low
- **Where:** `bot-modules/family/src/components/marry.rs:8-33` (`accept`),
  `bot-modules/family/src/components/adopt.rs:8-37` (`accept`). The guards
  (`MAX_PARTNERS`, "already adopted", "already related") live only in the
  *command* (`marry.rs:44-63`, `adopt.rs:46-60`), evaluated at proposal time.
- **What:** Between proposal and accept, state can change; the accept handler
  blindly `add_partner`/`add_child` + additive save with no recheck.
- **Failure scenario:** X sends `/marry @Z`; Y sends `/marry @Z` (both pass the
  command-time check because Z has 0 partners). Z clicks accept on both. Each
  accept adds a distinct partner via `ON CONFLICT DO NOTHING` on different pairs,
  so Z ends with 2 partners despite `MAX_PARTNERS = 1`. Same shape for two pending
  adoptions of one free child → child gets two parents.
- **Why it matters:** Invariant (`MAX_PARTNERS = 1`, single-parent adoption) is
  bypassable by anyone with two pending proposals; low severity because it needs
  cooperating/duplicate proposals and only corrupts the social graph, not economy.
- **Confidence:** confirmed (logic traced; no recheck exists).
- **Suggested fix:** Re-validate the invariants inside `accept` within the same
  transaction as the write (or make the write conditional: insert only if the
  partner/parent count is still under the cap), mirroring the CC-9 remediation.
