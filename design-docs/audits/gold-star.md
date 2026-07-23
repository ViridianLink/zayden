# Audit: gold-star

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Small (~344 LOC) star-giving feature. Concentrates two CC themes in one place:
the DB-generic `async_trait` manager (CC-1) **and** the runtime `sqlx::query(...)`
bypass (CC-5) in its `bot/` binding. No `tests/`. Because it is small and hits
both the abstraction and the SQL-style issues, it is the ideal **pilot** for the
concrete-`PgPool` + compile-time-macro migration.

## Findings

### 1. Runtime `sqlx::query(...)` bypassing macros  ·  #1  ·  med
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-23):** Folded into the CC-1 concrete-`PgPool` migration (finding
  #2). The last runtime-SQL site — the `get_row` `SELECT` (formerly
  `sqlx::query_as::<_, GoldStarRow>("…").bind(…)` in `bot/src/bindings/gold_star.rs`)
  — is now a compile-time `sqlx::query_as!` in
  `GoldStarRow::get_row` (`bot-modules/gold-star/src/manager.rs`), with an explicit
  `last_free_star AS "last_free_star: Timestamp"` column type override for the
  `jiff_sqlx::Timestamp` field. The `save_row` INSERT was already retired to
  `query!` during DS-1. `.sqlx/` regenerated against an empty freshly-migrated
  container (one new cache entry). No runtime `sqlx::query(...)` remains in this crate.
- **Where:** `bot/src/bindings/gold_star.rs:83` (INSERT…ON CONFLICT) and the
  `SELECT` above it — hand-written `sqlx::query("…").bind(…)`.
- **What / Why / Fix:** See [CC-5](_cross-cutting.md#cc-5). Convert to `query!`
  and regenerate `.sqlx/`.

### 2. DB-generic `async_trait` manager  ·  #1  ·  med
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-23):** Dropped the `#[async_trait] trait GoldStarManager<Db:
  Database>` and its lone `impl … for GoldStarTable` binding. The SQL now lives in
  the crate as concrete `PgPool` associated functions on `GoldStarRow`
  (`get_row`, `give_star`) using `query!`/`query_as!`, mirroring
  `ticket::TicketRow` / `destiny2::db`. `GiveStar::run`/`Stars::run` lost their
  `<Db, Manager>` generics and now take `&PgPool`, calling `GoldStarRow::…`
  directly; `bot/src/bindings/gold_star.rs` is reduced to the two `ModuleCommand`
  shims. Removed the now-unused `async-trait` dependency (`cargo machete` clean)
  and the `GoldStarManager` export from `lib.rs`. Behaviour-preserving: the moved
  `query!` bodies are byte-identical, so their existing `.sqlx` cache entries are
  reused unchanged. This is the CC-1 pilot; `levels` is the next-smallest.
- **Where:** `src/manager.rs`, `src/commands/{give_star,stars}.rs`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1). Migrate together with
  finding #1 in a single small PR — this crate is the recommended pilot.

### 3. No integration tests  ·  #6  ·  low
- **Where:** no `tests/` directory.
- **Suggested fix:** Add coverage for the free-star cooldown / star-count logic.
  See [CC-6](_cross-cutting.md#cc-6).

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lens: concurrency/atomicity. Instance of
[CC-9](_cross-cutting.md#cc-9); both directions of the race are present because
**both** the author and target rows are persisted with an absolute
`save_row` upsert (`bot/src/bindings/gold_star.rs:82-100`)._

### DS-1. `/give_star` read-modify-write races → star mint, loss, and free-star cap bypass  ·  Pass 2  ·  med
- **Status:** `complete — 82f308a2`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-19):** Replaced the read-mutate-two-absolute-saves flow with a
  single transactional `GoldStarManager::give_star` (`bot/src/bindings/gold_star.rs`).
  It ensures the author row exists, takes a `FOR UPDATE` lock on it, decides
  free-vs-paid in SQL (`last_free_star + INTERVAL '24 hours' <= now()`), spends the
  star with a guarded atomic write (`number_of_stars = number_of_stars - 1 WHERE
  number_of_stars >= 1`, `rows_affected == 1` asserted) or sets `last_free_star =
  now()` for a free star, then credits the target with an atomic `+ 1` upsert,
  returning the target's new total. This closes all three sub-races (author mint,
  target lost-update, free-star cap bypass). The command no longer reads/mutates
  rows in memory; the unused absolute `save_row`/in-memory mutators were removed.
  (New `query!` macros also retire the CC-5 runtime SQL in `save_row`; the `get_row`
  `SELECT` remains runtime-SQL under finding #1/CC-5.)
- **Where:** `bot-modules/gold-star/src/commands/give_star.rs:40-66`;
  `GoldStarRow::give_star`/`give_free_star`
  (`bot-modules/gold-star/src/manager.rs:35-49`); absolute upsert in
  `bot/src/bindings/gold_star.rs:82-100`.
- **What:** The command reads `author_row` and `target_row`, mutates both in
  memory (`-1`/`+1`), then writes each back with an absolute upsert
  (`number_of_stars = EXCLUDED.number_of_stars`). No transaction, no row lock, no
  conditional write. Three distinct failures fall out:
  - **Mint (author):** author has `number_of_stars = 1`. Two `/give_star` to
    different targets X and Y in the same tick. Both read author = 1, both pass
    `number_of_stars < 1 && !free_star`, both `give_star` → author = 0, X += 1,
    Y += 1. Both save author = 0. Author spent 1 star but handed out **2** → one
    star created.
  - **Loss (target):** the same target receives stars from authors A and B
    concurrently. Both read target = 5, both set 6, both absolute-save target = 6.
    Target should have 7 → **one received star silently lost**.
  - **Free-star cap bypass:** author has 0 stars and the 24h cooldown has
    elapsed. Two `/give_star` to two targets (or alts) in the same tick both read
    `last_free_star = old`, both compute `free_star = true`, both `give_free_star`
    → **2 free stars given in one day**, `last_free_star` recorded once.
- **Suggested fix:** wrap the whole operation in one transaction and use
  conditional/atomic writes: debit the author with
  `UPDATE gold_stars SET number_of_stars = number_of_stars - 1 WHERE id = $1 AND
  number_of_stars >= 1` (assert `rows_affected == 1`), credit the target with
  `... received_stars = received_stars + 1, number_of_stars = number_of_stars +
  1`, and gate the free star with a conditional `last_free_star` write. Fold into
  the CC-1/CC-5 concrete-`PgPool` migration for this crate. **Confidence:
  confirmed.**

## Clean
- #1 Architecture: simple manager + commands split.
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
