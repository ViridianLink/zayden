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
- **Status:** `complete — 0d2e0a0a`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-24, task 2 of 2 — dashboard read-view):** Built the web
  leaderboard the backend (task 1) unblocked. New `dashboard/src/server/levels.rs`
  `get_leaderboard(guild, global, page)` server fn reads `guild_levels` (guild
  scope) or `levels` (global scope) directly with `query_as!` — the SQL strings
  are kept **byte-identical** to `levels::LeaderboardRow::{guild,global}_leaderboard`
  so the shared `.sqlx` offline cache is reused (`git status .sqlx` clean, no
  regeneration; offline SSR build verified). It gates on `guild_admin_context`
  (same as every other `/guild/:id/*` fn, both scopes) and resolves display
  names/avatars best-effort via the bot's twilight client (falls back to the raw
  id on lookup failure — no serenity/bot-module dep pulled into the web crate).
  New `LevelsPage` (`ui/pages/levels.rs`) renders a paged table with a
  guild/global segmented toggle (resets to page 1 on flip; "Next" gated on a full
  page); routed at `/guild/:id/levels`, linked from the guild sidebar
  (`trophy` icon). `LeaderboardEntry` DTO + leaderboard/segmented/pager CSS in
  `style/partials/components.css`. No test (DB + Discord + WASM UI, no dashboard
  lib test target — see [CC-6](_cross-cutting.md#cc-6)); task 1's `tests/logic.rs`
  already covers the XP-curve math. **Residual:** per-page name resolution is
  ≤10 sequential Discord calls (a `users`-table name cache / bulk member fetch is
  a later optimisation, not a correctness issue).
- **Progress (2026-07-24, task 1 of 2 — levels dual-scope backend):** Reframed per
  the guild+global scope decision. The global `levels` table is unchanged; a new
  `guild_levels (guild_id, user_id PK)` table (migration `0017`) tracks per-guild
  XP, dual-written on every guild message with an **independent** per-scope
  cooldown and level curve (`accrue_message` shared helper). The level-up **coin
  reward stays global-only** (unchanged economy). `/rank`, `/xp`, `/leaderboard`
  gained an optional `global` param (guild default; auto-global outside a guild);
  the leaderboard is now a real `WHERE guild_id` query
  (`LeaderboardRow::guild_leaderboard`, with a `global_leaderboard` sibling) that
  **drops the `GuildMembersCache` filtering hack** — which also removes the
  member-list blocker for the dashboard view. New `guild_levels` starts empty, so
  a server board is blank until members chat post-deploy. **Remaining (task 2):**
  the actual dashboard read-view (`dashboard` server fns + `/guild/:id/levels`
  page) — the original #8 subject.
- **Where:** `src/commands/levels.rs` (leaderboard), `src/commands/rank.rs`,
  `src/components/levels.rs` (pager).
- **What:** Paged, data-dense displays better suited to a web page than an embed
  with prev/next buttons (also the CC-7 `custom_id` pager lives here).
- **Why it matters:** A web leaderboard removes the pager component entirely and
  reads better.
- **Suggested fix:** Add read-only dashboard views; keep the message-XP accrual
  in-bot. See [CC-8](_cross-cutting.md#cc-8).

### 5. Unused `tokio` dependency → `cargo machete` is not clean workspace-wide  ·  #7  ·  low
- **Status:** `complete — 994fea89`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-27):** Deleted `tokio = { workspace = true }` from
  `bot-modules/levels/Cargo.toml:19`; `Cargo.lock` drops the corresponding `levels →
  tokio` edge. Confirmed still-unused before removing (no `tokio` token anywhere in
  `levels/src/` or `levels/tests/`), and `cargo +nightly check -p levels --all-targets`
  compiles unchanged after — the crate's async surface is all `async fn` in traits plus
  `serenity`/`sqlx` types, none of which need a direct `tokio` dep. **No test added:**
  removing an unreferenced dependency has no runtime-behaviour delta to regress, so the
  gate *is* the verification — `cargo machete` reported this crate **before** and reports
  `didn't find any unused dependencies` **after**. Gate green: `cargo machete` clean
  (workspace-wide, for the first time), `cargo +nightly clippy --workspace --all-targets
  -D warnings` clean, `cargo test` green, `cargo +nightly fmt --check` clean. No new
  `#[allow]`/`#[expect]`. No SQL change, so no `.sqlx` delta.
  **Residual:** none. This clears the last blocker on the `cargo machete` exit gate, so
  future tasks touching a dependency list can now report it cleanly.
- **Found:** 2026-07-27, while running the `cargo machete` gate for
  [lfg #2](lfg.md). Pre-existing on clean `main` (verified by stashing), unrelated
  to that task, so left unfixed.
- **Where:** `bot-modules/levels/Cargo.toml` (`tokio = { workspace = true }`).
- **What:** `tokio` is declared but referenced nowhere in `src/` or `tests/` —
  most likely a leftover of the CC-1 concrete-`PgPool` migration (`04a8ab2b`),
  which is when `lfg`'s equivalent unused `async-trait` was removed.
- **Why it matters:** `cargo machete` is a mandated exit gate
  ([`CLAUDE.md`](../../CLAUDE.md)); while it reports this, the gate cannot be
  reported clean by *any* task that touches a dependency list, so the signal is
  permanently muddied.
- **Suggested fix:** delete the line, then `cargo +nightly check -p levels` and
  `cargo machete`. One-line change; no behaviour delta expected.

## Clean
- #1 DB access: concrete impl uses compile-time `query!`/`query_as!`/
  `query_scalar!` (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: message-create XP path is non-blocking; no locks across `.await`.
