# Audit: levels

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Small (~715 LOC) and functional, but carries three of the workspace themes: the
DB-generic `async_trait` manager (CC-1) in a legacy-named `sqlx_lib.rs`,
component `custom_id` string routing (CC-7), and no `tests/`. Because it is
small, it is a good **first** CC-1 migration to prove the concrete-`PgPool`
pattern before the larger crates.

## Findings

### 1. `LevelsManager<Db>` generic trait in `sqlx_lib.rs`  ·  #1  ·  high
- **Where:** `src/sqlx_lib.rs` (`trait LevelsManager<Db: Database>`, `Pool<Db>`,
  `#[async_trait]`); concrete impl in `bot/src/bindings/levels/mod.rs`
  (`impl LevelsManager<Postgres> for LevelsTable`, using `query!`/`query_as!`).
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1). The file name
  `sqlx_lib.rs` is itself a legacy/off-convention name — rename to `manager.rs`
  as part of the migration.

### 2. Component `custom_id` string routing  ·  #4  ·  low
- **Where:** `src/components/levels.rs:36` (`match … custom_id.as_str()` for
  page navigation).
- **What / Why / Fix:** See [CC-7](_cross-cutting.md#cc-7).

### 3. No integration tests  ·  #6  ·  med
- **Where:** no `tests/` directory.
- **What:** XP curve / level-up threshold math (`level_up_xp`,
  `common/levels.rs`) is pure and trivially testable but untested.
- **Suggested fix:** Add `tests/` for the XP-curve math (fast win). See
  [CC-6](_cross-cutting.md#cc-6).

### 4. Leaderboard / rank are better as dashboard read-views  ·  #8  ·  low
- **Where:** `src/commands/levels.rs` (leaderboard), `src/commands/rank.rs`,
  `src/components/levels.rs` (pager).
- **What:** Paged, data-dense displays better suited to a web page than an embed
  with prev/next buttons (also the CC-7 `custom_id` pager lives here).
- **Why it matters:** A web leaderboard removes the pager component entirely and
  reads better.
- **Suggested fix:** Add read-only dashboard views; keep the message-XP accrual
  in-bot. See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 DB access: concrete impl uses compile-time `query!`/`query_as!`/
  `query_scalar!` (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: message-create XP path is non-blocking; no locks across `.await`.
