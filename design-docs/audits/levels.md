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
- **Status:** `complete — 04a8ab2b`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-23):** CC-1 concrete-`PgPool` migration (the second pilot after
  `gold-star`). Dropped the `#[async_trait] trait LevelsManager<Db: Database>`
  and its lone `impl … for LevelsTable` binding. The SQL now lives in the crate as
  concrete `PgPool` associated functions using `query!`/`query_as!`/`query_scalar!`
  (`sqlx_lib.rs` renamed to `manager.rs`): `LeaderboardRow::leaderboard`,
  `RankRow::get`/`RankRow::user_rank`, `XpRow::get`, `FullLevelRow::get`, and
  `FullLevelRow::save(self, pool)`. `Levels::run`/`run_components` and
  `create_embed` lost their `<Db, Manager>` generics (keeping only
  `Data: GuildMembersCache`); `Rank::rank`, `Xp::xp`, and the free
  `message_create` are now non-generic over `&PgPool`. `bot/src/bindings/levels`
  is reduced to `register` + the `ModuleCommand`/`ModuleComponent` shims
  (`LevelsTable` deleted). Removed the now-unused `async-trait` dependency
  (`cargo machete` clean). **Behaviour-preserving:** every `query!` string was
  moved byte-identically, so the existing `.sqlx` cache entries are reused
  unchanged (`git status .sqlx` clean — no regeneration needed). `levels` is the
  second CC-1 pilot; `reaction-roles`/`suggestions` are the next-smallest.
- **Where:** `src/sqlx_lib.rs` (`trait LevelsManager<Db: Database>`, `Pool<Db>`,
  `#[async_trait]`); concrete impl in `bot/src/bindings/levels/mod.rs`
  (`impl LevelsManager<Postgres> for LevelsTable`, using `query!`/`query_as!`).
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1). The file name
  `sqlx_lib.rs` is itself a legacy/off-convention name — rename to `manager.rs`
  as part of the migration.

### 2. Component `custom_id` string routing  ·  #4  ·  low
- **Status:** `complete — 04a8ab2b`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-23):** Introduced a `LevelsCustomId` enum
  (`src/components/custom_id.rs`) with a `const as_str` and a `FromStr` whose
  error preserves the previous `"unrecognized levels component id: …"` message.
  The three button ids (`levels_previous`/`levels_user`/`levels_next`) are now
  defined once (`as_str`) and parsed once: `commands/levels.rs` builds the pager
  buttons from `LevelsCustomId::{Previous,User,Next}.as_str()`, and
  `components/levels.rs` routes on `custom_id.parse::<LevelsCustomId>()?` — a typo
  is now a compile error in the button builders, and the unreachable string
  fallback arm was removed (the `FromStr` `?` covers the unknown-id case). Follows
  the temp-voice/LFG namespaced-id convention (`levels_` prefix). Covered by the
  new round-trip / unknown-id tests (finding #3).
- **Where:** `src/components/levels.rs:36` (`match … custom_id.as_str()` for
  page navigation).
- **What / Why / Fix:** See [CC-7](_cross-cutting.md#cc-7).

### 3. No integration tests  ·  #6  ·  med
- **Status:** `complete — 04a8ab2b`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-23):** Added `tests/logic.rs` (integration file, per the
  `tests/`-only convention) covering the pure logic: the `level_up_xp` XP curve
  (exact values + strictly-increasing across levels 0–100) and the new
  `LevelsCustomId` round-trip / namespacing / unknown-id rejection. DB-touching
  paths (`RankRow`/`XpRow`/`FullLevelRow` queries) are left for a future test-pool
  harness — see [CC-6](_cross-cutting.md#cc-6).
- **Where:** no `tests/` directory.
- **What:** XP curve / level-up threshold math (`level_up_xp`,
  `common/levels.rs`) is pure and trivially testable but untested.
- **Suggested fix:** Add `tests/` for the XP-curve math (fast win). See
  [CC-6](_cross-cutting.md#cc-6).

### 4. Leaderboard / rank are better as dashboard read-views  ·  #8  ·  low
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
  (Direction finding, not a defect — building Leptos dashboard read-views is a
  scoped [CC-8](_cross-cutting.md#cc-8) feature task, out of scope for the
  levels-module fix pass. Left for deliberate scheduling.)
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
