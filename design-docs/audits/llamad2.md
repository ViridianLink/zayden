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
- **Status:** `complete — f2cf893d`            <!-- open | in-progress | in-review | complete | wontfix -->
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
- **Status:** `complete — f2cf893d` (folded into finding #1)            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** same as #1 (`countingFails.json`, `dumbCount.json`).
- **What / Why / Fix:** Persistent per-guild counters stored outside the DB.
  Fold into the fix for #1. **Resolved by finding #1's fix** — both counters now
  live in the `llamad2_counters` DB table (migration `0016_llamad2_counters`).

### 3. No integration tests  ·  #6  ·  low
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-31):** Added `tests/counters.rs` (5 tests, DB-backed) and
  `tests/triggers.rs` (9 tests, offline). Split by what the code actually is:
  - **The counters are the high-value target**, because finding #1's fix — the
    move to an atomic server-side `count = llamad2_counters.count + 1` — landed
    with **no test** (its own note says so: no live-`PgPool` harness existed
    then). `gold-star` built one since (`9a7b8795`), so these are that fix's
    missing net. `#[sqlx::test(migrations = "../../migrations")]`, no fixture —
    an empty `llamad2_counters` is itself the first case, since the flat-file
    version errored on a freshly-created empty file (`serde_json::from_str("")`)
    and dropped the very first fail of a fresh deploy.
  - **Extraction (approved at task selection):** the upsert was inlined
    *byte-identically* in `counting_fail.rs` and `goof.rs`, inside `run()`
    methods taking `&Context`, so it was unreachable from a test. Both now call
    a new `Counter::bump(pool, name)` (`src/counter.rs`), which also
    de-duplicates the statement and gives the two counter names — the table's
    primary key — a single home as `Counter::{COUNTING_FAILS, DUMB_COUNT}`.
    The SQL string is preserved character-for-character, so its `.sqlx` entry
    (`query-325714ad….json`) still resolves and the cache is **unchanged** — the
    offline build passing is the proof.
  - **Offline half:** the three predicates that decide whether a handler acts at
    all. `is_good_morning` and a new `should_greet` (the alternation gate, lifted
    out of `GoodMorning::run`'s `if`) and `is_codeword` (lifted out of
    `BehindTheScenes::run`) are now `pub` and re-exported. `is_codeword` is the
    one worth the most: it gates an `add_member_role`, and its whole-message
    `eq_ignore_ascii_case` rule is what stops the role being farmed by pasting a
    wordlist into the channel.
  - **Deliberately uncovered:** `/hello`, `/socials`, `/playlist`,
    `/sensitivity`, `/raidreport`, `/dungeonreport` are each a constant string
    sent to Discord, and `StatusUpdate` is a presence match with no logic to
    pin. Asserting a literal equals itself is the trivia checklist #6 warns
    against.
  - Making the predicates public raised three `clippy::must_use_candidate`
    errors and one `doc_markdown`; all four were **fixed** (`#[must_use]`,
    backticks), not suppressed. No new `#[allow]`/`#[expect]`.
- **Verification (mutation testing).** The code under test was already correct,
  so — per [CC-6](_cross-cutting.md#cc-6)'s pattern — fails-before was
  established by breaking each property in turn and re-running (each reverted;
  `git diff` confirms `src/` carries only the extraction):

  | Mutation | Result |
  |---|---|
  | `DO UPDATE SET count = llamad2_counters.count + 1` → `= EXCLUDED.count` (the pre-fix absolute write) | 2 of 3 DB tests fail — concurrency one with `[1, 1]` vs `[1, 2]` |
  | `VALUES ($1, 1)` → `VALUES ($1, 0)` | all 3 DB tests fail |
  | `ON CONFLICT (name)` arm dropped | 2 of 3 DB tests fail — `23505` unique violation |
  | `Counter::COUNTING_FAILS` renamed | `counter_names_are_the_persisted_ones` fails |
  | `is_good_morning`: `starts_with` → `contains` | `the_greeting_match_is_a_prefix_of_the_trimmed_message` fails |
  | `should_greet`: `last_author != author` dropped | `a_greeting_after_your_own_greeting_is_not_answered` fails |
  | `is_codeword`: `eq_ignore_ascii_case` → `to_lowercase().contains` | `codewords_match_the_whole_message_case_insensitively` fails |

  Gate green against a throwaway Postgres 18: `cargo +nightly clippy --workspace
  --all-targets -- -D warnings` clean, `cargo test` 529 passed / 0 failed,
  `cargo machete` clean (the `Cargo.toml` delta is `sqlx`'s `migrate` feature
  plus a `tokio` dev-dependency for `join!`). `.sqlx/` unchanged.
- **Behaviour pinned, not changed:** `is_good_morning` matches a **prefix**, so
  `"gm"` matches `"gmail is down"`. Left as-is and asserted both ways — the
  alternation gate is what keeps it from producing a stray reply — rather than
  silently changing which messages the bot answers. Worth its own finding if the
  owner wants it tightened.
- **Where:** ~~no `tests/` directory~~ → `tests/counters.rs`, `tests/triggers.rs`.
  Low priority — mostly cosmetic handlers.

## Clean
- #1 Architecture: one file per handler; simple.
- #4 Stringly typing: handler dispatch is in `bot/` bindings; nothing egregious.
- #7 Lint: no `#[expect]`/`#[allow]` in this crate.
