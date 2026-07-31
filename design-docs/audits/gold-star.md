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
- **Status:** `complete — c2b4c4cf`            <!-- open | in-progress | in-review | complete | wontfix -->
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
- **Status:** `complete — c2b4c4cf`            <!-- open | in-progress | in-review | complete | wontfix -->
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
- **Status:** `complete — 9a7b8795`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-29):** Added `tests/give_star.rs` (5 tests) + `tests/fixtures/
  gold_stars.sql`. This crate is a thin shell over SQL and Discord I/O — the
  free-star window, the overdraft floor and the atomic credit all live inside
  `GoldStarRow::give_star`'s single transaction (`manager.rs:50-120`), with no
  Rust-side representation to assert against. The workspace's existing tests are
  all pure/offline, so on the owner's ruling this task **built the workspace's
  first DB-backed harness** rather than settle for asserting `GoldStarRow::new`
  defaults:
  - `sqlx = { features = ["migrate"] }` + `#[sqlx::test(migrations =
    "../../migrations", fixtures("gold_stars"))]`. Each test gets its own
    freshly-migrated database, created and dropped by sqlx; migrations are
    applied by the harness, so no `sqlx migrate run` is involved.
  - `tokio` added as a **dev**-dependency for `join!` (the concurrency test).
  - Coverage: the free star is free and closes its own 24h window; the paid arm
    debits exactly one and refuses at zero without crediting; a refused give
    commits nothing (not even the target's row); and two concurrent gives to one
    target both land.
- **Verification (mutation testing).** The code under test was already fixed by
  [DS-1](#ds-1-give_star-read-modify-write-races--star-mint-loss-and-free-star-cap-bypass--pass-2--med),
  so "fails-before" was established by removing each guard in turn and re-running
  (each then reverted; `src/` is unmodified by this task):

  | Guard removed | Result |
  |---|---|
  | free-star comparison `<=` flipped to `>=` | 4 of 5 fail |
  | `last_free_star = now()` bump (the cap bypass) | `free_star_is_free_and_closes_its_window` fails |
  | atomic credit → absolute `EXCLUDED.number_of_stars` (the DS-1 shape) | `concurrent_gives_to_one_target_both_land` fails, `[1, 1]` vs `[1, 2]` |
  | `WHERE number_of_stars >= 1` **alone** | **nothing fails** |
  | all three overdraft guards together | 3 of 5 fail |

  The fourth row is the interesting one and is recorded in the test's own doc
  comment: the app-layer balance check, the SQL floor and the `rows_affected`
  assertion are **mutually redundant in a single process**, because the
  `FOR UPDATE` read already excludes a concurrent balance change. The floor is
  defence-in-depth against the lock being weakened, not independently
  observable — so no test should claim to cover it.
- **Where:** ~~no `tests/` directory~~ → `tests/give_star.rs`,
  `tests/fixtures/gold_stars.sql`.
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

### DS-2. `give_star` never ensures a `users` row → FK violation for an unseen author or target  ·  Pass 10 (CC-6 harness) / #1  ·  low-med
- **Status:** `complete — 0104f142`            <!-- open | in-progress | in-review | complete | wontfix -->
  _Reconciled 2026-07-31: the marker was left at `in-review` by the fix's own
  commit. `0104f142` ("ensure users are created for both author and target in
  `give_star`") is that commit — it carries `manager.rs`, `commands/give_star.rs`,
  the two `tests/give_star.rs` regressions, the fixture change and one new
  `.sqlx` entry, matching the fix note below._
- **Fix (2026-07-30):** `GoldStarRow::give_star` now inserts the `users` row for
  **both** actors inside its existing transaction, immediately before each
  `gold_stars` write — `INSERT INTO users (id, username) VALUES ($1, $2) ON
  CONFLICT (id) DO NOTHING` — mirroring the `levels`/`family` idiom
  (`manager.rs:66-73` author, `:122-129` target). Real Discord usernames are
  threaded through rather than a placeholder: the signature gained
  `author_name`/`target_name`, fed from `interaction.user.name` and the resolved
  `target_user.name` in `commands/give_star.rs`, so a member first seen by
  `/give_star` gets a correct `users.username` instead of `levels`'
  `'PLACEHOLDER'`. `DO NOTHING` (not `DO UPDATE`) so an existing row maintained
  by another module is never clobbered — see the residual note below.
  Both macros are textually identical, so `.sqlx/` gained a single new entry.
- **Verification:** two `#[sqlx::test]` regressions in `tests/give_star.rs`,
  `an_unseen_author_can_give` and `an_unseen_target_can_be_given_to`, using ids
  500/950 that `tests/fixtures/gold_stars.sql` deliberately omits. Fails-before
  established by mutation, one insert removed at a time:

  | Insert removed | Result |
  |---|---|
  | author's `INSERT INTO users` | `an_unseen_author_can_give` fails — `23503`, `Key (id)=(500) is not present in table "users"` |
  | target's `INSERT INTO users` | `an_unseen_target_can_be_given_to` fails — `23503`, `Key (id)=(950)` |

  Both mutations were reverted; the other 5 tests stay green under each. The
  persisted username was confirmed to be the real one (`500` → `unseen-author`,
  not a placeholder) with a throwaway probe, not asserted in the committed suite:
  the crate has no `users` reader, and a `query!` in a test binary would need a
  `.sqlx` cache prepared with `--all-targets`, which the workspace's prepare
  command does not use.
- **Found:** 2026-07-29, while writing the [#3](#3-no-integration-tests--6--low)
  harness — the first fixture attempt failed on the FK, which is how the gap
  surfaced. Recorded separately per one-finding-one-task.
- **Where:** `src/manager.rs:57-64` (the author's `INSERT INTO gold_stars …
  ON CONFLICT DO NOTHING`) and `:105-115` (the target's upsert). Schema:
  `migrations/0001_v1_init.up.sql:132` — `gold_stars.id BIGINT PRIMARY KEY
  REFERENCES users (id)`.
- **What:** `gold_stars.id` is a foreign key into `users`, but `give_star`
  inserts into `gold_stars` without ensuring the `users` row exists. Compare
  `levels/src/manager.rs:320,437` and `family/src/manager.rs:286,421`, which
  both `INSERT INTO users (id, username) … ON CONFLICT (id) DO NOTHING` first.
- **Why it matters:** `/give_star` against a user with no `users` row fails with
  a `23503` foreign-key violation. That surfaces as `GoldStarError::Sqlx`, whose
  `Respond::user_message` returns `None` (`error.rs:52`), so the user gets a
  generic failure with no indication of why. In practice most members already
  have a `users` row from levels XP accrual, which is why this has not been
  loud — a member who has never sent a message is the reachable case.
- **Suggested fix:** mirror the `levels`/`family` idiom — insert the `users` row
  for **both** author and target inside the existing transaction, before the
  `gold_stars` writes. The command layer has the `User` objects, so a real
  username can be passed rather than a placeholder.
- **Residual:** `users.username` is written on insert only. A member whose row
  was created earlier by `levels` keeps `'PLACEHOLDER'`; `/give_star` does not
  refresh it. Refreshing every give would add a write to a row other modules
  maintain, so the stale-username question is left as its own concern — the
  workspace has no single owner for `users.username` today (`family::save` does
  `DO UPDATE`, `levels` does not).

## Clean
- #1 Architecture: simple manager + commands split.
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
