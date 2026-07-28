# Audit: gambling

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

The largest module (~10.7k LOC, 63 src files) and the heaviest concentration of
the workspace themes: the DB-generic `async_trait` manager pattern is pervasive
here (CC-1), it holds most of the `#[expect]` cluster (CC-3), a self-described
dead `GameState` stub (CC-4), an inline test module (CC-2), and the deferred
component-`custom_id` string routing (CC-7). Coverage is thin (1 `tests/` file)
for the size. No runtime-SQL — the concrete impls in `bot/src/bindings/gambling`
use compile-time macros.

## Findings

### 1. DB-generic `async_trait` managers (pervasive)  ·  #1  ·  high
- **Status:** `complete — 83930148`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-24):** CC-1 concrete-`PgPool` migration — the final and largest
  module (after `gold-star`/`levels`/`reaction-roles`/`suggestions`/`family`/
  `temp-voice`/`lfg`). Removed all 21 generic manager traits + `#[async_trait]`
  (`GamblingManager`, `GameManager`, `StatsManager`, `EffectsManager`,
  `InventoryManager`, `Stamina/Lotto/Prestige/Daily/Goals/Work/Craft/Dig/Mine/
  Gift/Send/Profile/Leaderboard/ShopManager`, `HigherLowerManager`) and their 23
  `impl … for XxxTable` bindings. Each became a concrete unit-struct (or row-type)
  with inherent `&PgPool`/`&mut PgConnection` methods living in the crate; the
  `GameManager`/`ProfileManager`/`MineManager` row ops moved onto
  `GameRow`/`ProfileRow`/`MineRow` (`::get`/`::save`). Every command/game/
  component/event lost its `<Db, …Handler>` generics (keeping only `Data:` ctx
  bounds); `Dispatch` de-generic'd; the `StaminaCron`/`Lotto`/`HigherLower` crons
  pinned to `Postgres` (the zayden-core `CronJob<Db>` generic is its own CC-1
  item, untouched). The 27 `PostManager`-style SQL files moved
  `bot/sql/gambling/` → `bot-modules/gambling/sql/` (byte-identical → offline
  cache reused; `git status .sqlx` clean, no regeneration). `bot/src/bindings/
  gambling/*` collapsed to `ModuleCommand`/`ModuleComponent` wiring only —
  `models.rs`/`stamina.rs` deleted (were table-only). Dropped `async-trait` +
  added `bigdecimal` (for lotto `total_tickets` SUM) in the crate; removed
  now-unused `bigdecimal`/`jiff-sqlx` from `bot`. Also deleted the dead
  `tictactoe` `GameState<Db, Manager>` stub ([CC-4](_cross-cutting.md#cc-4)) — it
  referenced the removed `GameManager` trait — clearing 3 `#[expect]`s. No new
  `#[allow]`/`#[expect]`. Only the `zayden-core` generalising traits now remain
  on CC-1.
- **Where:** `src/models/*`, `src/commands/*`, `src/games/*`, `src/common/*`,
  `src/components/*` — nearly every file threaded `<Db: Database>` / `Pool<Db>`;
  concrete impls lived in `bot/src/bindings/gambling/*`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1). This is the biggest
  single instance and should be migrated last (largest surface); tackle it after
  the small crates prove the pattern.

### 2. Dead `GameState` / reserved stubs  ·  #2  ·  low
- **Where:** `src/commands/tictactoe.rs:175,182`
  (`future_not_send, reason = "dead code within GameState stub"`),
  `src/common/shop/items.rs:192` (`dead_code, reason = "reserved for future
  implementation"`).
- **What / Why / Fix:** See [CC-4](_cross-cutting.md#cc-4). Delete the stub
  until the feature is real; removes two CC-3 escape-hatches with it.

### 3. `#[expect]` cluster  ·  #7  ·  med
- **Where:** `src/utils.rs:85`, `src/models/mod.rs:74` (`cast_sign_loss`),
  `src/commands/tictactoe.rs:136,151`, `src/commands/gift.rs:37`,
  `src/common/shop/items.rs:47`, `src/games/lotto.rs:118`.
- **What / Why / Fix:** See [CC-3](_cross-cutting.md#cc-3). The
  `cast_sign_loss` on stamina points at a domain-type opportunity (an unsigned
  stamina type) rather than a cast suppression.

### 4. Inline `#[cfg(test)]` + string-routed components  ·  #6 / #4  ·  med / low
- **Where:** `src/components/tictactoe.rs:509` (inline test — CC-2);
  `src/components/{tictactoe,higherlower}.rs` + `bot/src/bindings/gambling/
  {prestige,blackjack}.rs` (`custom_id.as_str()` routing — CC-7).
- **What / Why / Fix:** See [CC-2](_cross-cutting.md#cc-2) and
  [CC-7](_cross-cutting.md#cc-7).

### 5. Thin test coverage for size  ·  #6  ·  med
- **Where:** one `tests/` file vs. 63 src files of money/economy logic.
- **Why it matters:** Economy math (payouts, prestige, stamina, lotto odds) is
  exactly the logic that should be pinned by tests before a refactor.
- **Suggested fix:** Add pure-logic `tests/` for payout/odds/stamina math ahead
  of the CC-1 migration.

### 6. Leaderboard / profile are better as dashboard read-views  ·  #8  ·  low
- **Where:** `src/commands/leaderboard.rs`, `src/commands/profile.rs` (+ their
  `components/*` pagers).
- **What:** Data-dense, paged displays that a Discord embed renders poorly
  (button pagination, field limits).
- **Why it matters:** A web leaderboard/profile page is a strictly better view and
  offloads the pager-component complexity.
- **Suggested fix:** Add read-only dashboard views; **keep all games/economy
  actions in-bot** (they are live interactions). Lower priority than
  de-duplicating config writes. See [CC-8](_cross-cutting.md#cc-8).

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lenses: silent-failure, concurrency/atomicity, SQL
integrity, drift. See [CC-9](_cross-cutting.md#cc-9) for the workspace-wide
read-modify-write race class this drills beneath (CC-1 enables it)._

### DS-1. `/send` is a non-atomic, racy transfer → coins minted from nothing  ·  Pass 1+2  ·  high
- **Status:** `complete — d2108103`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/gambling/src/commands/send.rs:146-190`; SQL semantics in
  `bot/src/bindings/gambling/models.rs:70-84` (`add_coins` = atomic
  `coins = gambling.coins + $2`) vs. `send.rs:47-60` (`save` = absolute
  `coins = EXCLUDED.coins`).
- **What:** The recipient is credited with an **atomic increment** inside a
  committed tx (line 166-170), then — *after* a fallible `Dispatch::fire`
  (line 182-188) — the sender is debited via an **absolute** row overwrite
  (line 190). Debit and credit are two separate transactions with fallible
  HTTP+DB work between them, and the debit is a read-modify-write with no row
  lock / `WHERE coins >= amount` guard.
- **Failure scenario (two independent bugs):**
  1. *Concurrency:* sender has 100 coins. Fire two `/send 100 @x` in the same
     tick. Both read `coins = 100`, both pass `coins < amount` (100 < 100 =
     false), both `add_coins(@x, 100)` (atomic → @x gains **200**), both
     `save(sender coins = 0)`. Sender lost 100, recipient gained 200 → **100
     coins created**.
  2. *Partial application:* `Dispatch::fire` (line 182) returns `Err` (goal DB
     write hiccup, or the channel send 404s) *after* the recipient credit already
     committed at line 170. Function returns `?` before line 190, so the sender is
     never debited → recipient keeps the coins, sender keeps their balance.
- **Suggested fix:** Do the whole transfer in one transaction: `UPDATE gambling
  SET coins = coins - $amt WHERE user_id = $sender AND coins >= $amt` (check
  `rows_affected == 1`), then `add_coins(recipient)` on the *same* tx, commit,
  and only then fire non-critical Dispatch/embeds. **Confidence: confirmed.**

### DS-2. `/gift` daily limit bypassed by double-submit → double free mint  ·  Pass 2  ·  high
- **Status:** `complete — fb5b9c7c`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/gambling/src/commands/gift.rs:166-198`.
- **What:** Gift mints `GIFT_AMOUNT * (prestige+1)` free coins to the recipient
  via atomic `add_coins` (committed, line 178-182); the once-per-day guard is a
  read of `user_row.gift` date (line 172) whose new value is only persisted by the
  absolute `save_sender` at line 198. Classic check-then-act with no lock.
- **Failure scenario:** user fires two `/gift @alt` in the same tick. Both read
  `gift = yesterday`, both pass the `== now.date()` check, both `add_coins(@alt,
  amount)` (atomic → @alt gains **2×amount** of newly-minted coins), both
  `save_sender(gift = today)`. The daily cap recorded once, but the mint happened
  twice → a user can inject 2× the intended free coins into the economy every day
  (to an alt they control).
- **Suggested fix:** Gate the mint on a conditional write:
  `UPDATE gambling SET gift = now() WHERE user_id = $1 AND (gift IS NULL OR
  gift::date < current_date)` and only credit the recipient when
  `rows_affected == 1`, all in one tx. **Confidence: confirmed.**

### DS-3. Prestige→lotto `ON CONFLICT` computes `2×tickets` and discards the pool  ·  Pass 4+9  ·  med
- **Status:** `complete — 82f308a2`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-19):** `lotto.sql` upsert now reads the existing pool row —
  `SET quantity = gambling_inventory.quantity + $3` (mirrors `add_coins.sql`'s
  `coins = gambling.coins + $2`) instead of `EXCLUDED.quantity + $3` (= `$3 + $3`,
  which ignored the accumulated pool and wiped/doubled it).
- **Where:** `bot/sql/gambling/PrestigeManager/lotto.sql` (used by
  `bot/src/bindings/gambling/prestige.rs:53-66`, called from
  `bot-modules/gambling/src/commands/prestige.rs:303-314`).
- **What:** The upsert body is
  `SET quantity = EXCLUDED.quantity + $3`. In Postgres `EXCLUDED.quantity` is the
  *proposed insert value* = `$3` (the prestiger's ticket count), so on conflict
  the house-pool row (`zayden_id`'s `gambling_inventory` LOTTO_TICKET) is set to
  `$3 + $3 = 2×tickets` — the **existing accumulated pool is never read**. Every
  other upsert in the module correctly references the table row
  (`add_coins.sql`: `coins = gambling.coins + $2`); this one diverged.
- **Failure scenario:** house pool holds 10 000 tickets from prior prestiges. A
  user with **0** lotto tickets prestiges (the common case — most prestiges hold
  none): `Manager::lotto` runs with `tickets = 0`, conflict fires,
  `quantity = 0 + 0 = 0` → **entire accumulated lotto pool wiped to zero**.
  Alternatively a whale prestiges with 5 000 tickets → pool jumps to 10 000
  (2×), inflating the Friday jackpot's `total_tickets` and thus every real
  winner's payout. Either way the pool value is corrupt (should be
  `old + tickets`).
- **Suggested fix:** `SET quantity = gambling_inventory.quantity + $3`.
  **Confidence: confirmed** (Postgres `EXCLUDED` semantics + the correct sibling
  upsert).

### DS-4. `confirm_prestige` button has no double-submit idempotency  ·  Pass 2  ·  med
- **Status:** `complete — 82f308a2`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-19):** Took the finding's second suggested fix (optimistic
  concurrency), which is race-safe where "ack-first" is not. `PrestigeManager::
  save` now takes the pre-increment `expected_prestige` and its `gambling_mine`
  write is a guarded `UPDATE ... SET prestige = N+1, … WHERE user_id = $1 AND
  prestige = $expected` (plain UPDATE, not an upsert — reaching confirm requires
  an existing mine row, so a 0-row result is a lost race, not a missing row). It
  returns `bool`; on `rows_affected != 1` the transaction is rolled back (so the
  coins/inventory writes never land) and `false` is returned. `confirm_prestige`
  now (a) captures the lotto-ticket count *before* the write clears the inventory,
  (b) calls `save` first, and (c) only calls `Manager::lotto(...)` when `save`
  returned `true`. A same-tick double-click: both pass the stale miner check, both
  compute `prestige = N+1`, but only the first `UPDATE` matches (`prestige = N`);
  the second matches 0 rows → rolls back → returns a stale-confirmation error and
  **never contributes to the lotto pool a second time**. **Residual:** `lotto` and
  `save` remain separate transactions (a `lotto` failure after a committed `save`
  loses that one contribution) — folding both into one tx is CC-1 work. No
  regression test (concurrency + DB, no lib target — see gold-star/lfg DS-1).
- **Where:** `bot-modules/gambling/src/commands/prestige.rs:269-333`;
  routing at `bot/src/bindings/gambling/prestige.rs:214-223`.
- **What:** The confirm handler re-reads the row and re-checks `miners >=
  req_miners` (good), but the buttons are only removed by the `UpdateMessage`
  *after* all DB writes (line 320). Two clicks in the same tick both pass the
  miner check and both run `Manager::lotto(...)` before either response lands.
- **Failure scenario:** user double-clicks Confirm with 50 lotto tickets. Both
  invocations read `miners = req`, both call `Manager::lotto(pool, 50, ...)`.
  Combined with DS-3's broken upsert the second call still overwrites, but even
  with DS-3 fixed the tickets get contributed twice while the user only pays one
  prestige (both `save` write `prestige = N+1` absolutely). The
  `do_prestige`/`save` coin/gem/prestige changes are masked by the absolute
  overwrite, but the lotto-pool contribution is doubled.
- **Suggested fix:** Ack-and-disable the buttons first (`UpdateMessage` before the
  DB work), or gate the whole confirm on a single `UPDATE gambling_mine ... WHERE
  prestige = $expected` optimistic-concurrency check. **Confidence: confirmed**
  for the double-execution window; **plausible** for real-world exploitation
  (requires holding lotto tickets *and* a same-tick double click).

### DS-5. `bet` decrement has no `WHERE coins >= bet` guard → overdraft via cross-command race  ·  Pass 2+4  ·  med
- **Status:** `complete — 3a58df90`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot/sql/gambling/GamblingManager/bet.sql`
  (`UPDATE gambling SET coins = coins - $2 WHERE user_id = $1` — no balance
  floor); called from the wager games, e.g.
  `bot-modules/gambling/src/commands/blackjack.rs:83`.
- **What:** Sufficiency is checked at the app layer (`EffectsHandler::bet_limit`
  on a `coins` value read in a *separate, already-committed* tx at
  `blackjack.rs:65-69`), then the debit is an unconditional atomic decrement. The
  `game_cache` 5s guard (`game_cache.rs:12`) correctly blocks a *second game* from
  the same user, but it does **not** cover other balance-spending commands
  (`/send`, `/gift`, faucets), so the check and the debit can straddle another
  balance change.
- **Failure scenario:** user has 100 coins. `/blackjack 100` reads `coins = 100`
  and passes `bet_limit`. Before its `bet` decrement runs, a concurrent `/send 100
  @x` credits @x (+100 atomic) and absolute-saves the sender to `coins = 0`.
  `blackjack`'s decrement then runs: `0 - 100 = -100`. The user now holds a
  **negative balance**, kept the 100-coin bet in play, and @x received 100 →
  compounds DS-1. Nothing rejects the game because `bet.sql` never checks the
  floor.
- **Suggested fix:** make the debit conditional — `UPDATE gambling SET coins =
  coins - $2 WHERE user_id = $1 AND coins >= $2` and treat `rows_affected == 0` as
  insufficient funds — instead of relying on a stale app-layer read. **Confidence:
  confirmed** for the missing guard; **plausible** for the specific interleave.

### DS-6. Lotto cron rebuilds `WeightedIndex` after the *final* pick → whole draw rolls back at exactly 3 participants  ·  Pass 5  ·  med
- **Status:** `complete — a6b211ca`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/gambling/src/games/lotto.rs:110-125` (winner loop),
  error/rollback path at `:189-193`.
- **What:** The winner loop iterates over the 3 prize shares
  (`[0.5, 0.3, 0.2]`), and after every `rows.remove(index)` it **unconditionally**
  rebuilds `WeightedIndex::new(rows.iter().map(quantity))` (`:114-117`) for the
  next sample — *including after the last winner*. `WeightedIndex::new` errors on
  an empty iterator, and the closure maps that to `Err`, which propagates out of
  the `pool.begin()` transaction before `delete_tickets` (`:127`), the `add_coins`
  payout loop (`:141-157`), and `tx.commit()` (`:159`). The gate at `:98`
  (`rows.len() < expected_winners`) admits exactly-3 as valid.
- **Failure scenario:** A guild's weekly lotto has exactly 3 eligible participants
  (bot excluded at `:93`). The 3rd iteration removes the last winner, leaving
  `rows` empty; the trailing `WeightedIndex::new([])` returns `Err` →
  `error!("lotto cron job failed: …")` and the transaction is dropped (rolled
  back). No winners are paid, `delete_tickets` never runs, and the tickets carry
  into the next week. With exactly 3 players this repeats **every** Friday — the
  lotto silently never pays out.
- **Why it matters:** Small/new servers frequently sit right at 3 participants; for
  them the entire lottery feature is dead, and the failure is invisible (logs
  only, tx rolled back so no partial state to notice).
- **Confidence:** confirmed (traced the loop, the empty-`WeightedIndex` error, and
  the rollback boundary). **✅ Reproduced in production (2026-07-17):** the log
  `lotto cron job failed: Internal error: WeightedIndex update failed: Not enough
  weights > zero` is exactly this path — the trailing `WeightedIndex::new` at
  `lotto.rs:114-117` on an empty `rows`. (`WeightedIndex` reports the empty/all-zero
  case as "Not enough weights > zero".)
- **Suggested fix:** Only rebuild the distribution when another sample is needed —
  skip the rebuild on the final iteration, or guard the top of the loop with
  `if rows.is_empty() { break }`. The payout math is independent of the trailing
  rebuild, so it is pure dead work that also poisons the draw.

## Clean
- #1 DB access: concrete impls use compile-time `query!`/`query_as!` (no
  runtime SQL) — the CC-1 issue is the abstraction, not the queries.
- #3 Async: no blocking I/O; no locks across `.await` observed.
- Wager games (`/blackjack`, `/higherorlower`) are safe from **intra-game**
  double-submit: `GameCache::check_and_set` (`game_cache.rs`) atomically rejects a
  repeat within 5s, and the bet is taken via the **atomic** `bet` decrement (not
  an absolute save). The residual hazard is cross-command overdraft (DS-5), not
  double-execution.
- Self-credit faucets (`/daily`, `/work`) are **not** double-mintable: they
  read-modify-write the *same* user's row with an absolute `save`, so a
  concurrent double-submit lands `old + one_payout` (the lost update favours the
  house). The exploitable cases are the ones that credit a *different* user via
  atomic `add_coins` (DS-1, DS-2) or a shared pool (DS-3/DS-4).

### DS-7. `daily` / `work` are further CC-9 whole-row absolute-overwrite sites (lost concurrent update)  ·  Pass 2  ·  low-med
- **Status:** `complete — b5bfebf6` (`daily` half fixed in `82f308a2`; `work` half in `b5bfebf6`)            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-25, `work`):** The deferral above no longer applies — the
  [CC-1](_cross-cutting.md#cc-1) concrete-`PgPool` migration has landed, so the
  per-field-delta conversion is now a surgical change. `WorkManager::save` (whole-row
  absolute upsert) replaced with `WorkManager::commit_work`, a single guarded atomic
  upsert — `INSERT … ON CONFLICT DO UPDATE SET coins = gambling.coins + $2,
  gems = gambling.gems + $3, stamina = gambling.stamina - 1 WHERE gambling.stamina > 0
  RETURNING coins, gems, stamina`. The command now captures a pre-image right after its
  read and persists `WorkDelta::between(before, after)` — taken **after**
  `Dispatch::fire`, so goal rewards (`add_coins(5_000)`/`add_gems(1)`) ride the same
  atomic increment. No row returned ⇒ the guard rejected the decrement (another command
  spent the last stamina point in between) ⇒ `out_of_stamina()`, and the transaction
  rolls back without writing `mine_activity`.
  - **Also closes the stamina check-then-act race.** `verify_work()`/`done_work()` were
    a purely in-memory guard: two concurrent `/work` calls both read `stamina = 3` and
    both absolute-wrote `2`, so the second shift was free. The guarded decrement makes
    the two calls consume two points (3→2→1), the same corrective pattern as
    [DS-5](#ds-5).
  - The embed now reports the **post-write** balances from `RETURNING` rather than the
    in-memory pre-image plus this shift's earnings.
  - `out_of_stamina()` was extracted out of `Stamina::verify_work` (`models/mod.rs`) so
    the up-front check and the guarded write raise the identical error.
  - **New-row parity:** the INSERT branch still seeds `coins = delta`,
    `stamina = MAX_STAMINA - 1`, matching the old `WorkRow::new` + `done_work()` path.
  - **Verification:** `bot-modules/gambling/tests/work.rs` (4 tests) pins the
    delta-vs-absolute distinction at the value level — modelling the interleave where a
    +500 credit lands mid-shift and asserting `current + delta` = 1 800 while the old
    absolute write produced 1 300. Following the family DS-1 / lotto DS-6 precedent, the
    end-to-end path is not covered: it needs a live `PgPool` + Discord interaction and
    this crate has no DB harness (see [CC-6](_cross-cutting.md#cc-6)), and the guard
    itself lives in SQL. `WorkDelta` did not exist before the fix, so the test is
    structurally unbuildable against the old code rather than red-then-green.
  - **Gate:** `cargo +nightly clippy --workspace --all-targets -D warnings` clean,
    `cargo test` green (98 test binaries). No new `#[allow]`/`#[expect]` — the
    `#[expect(trivial_casts)]` on `commit_work` is the pre-existing one carried over
    from `save` (sqlx's `jiff_sqlx` bind-type override). No `Cargo.toml` dep change, so
    no `cargo machete` run. `.sqlx`: one new entry (`176a9f01`).
  - **Residual:** `Dispatch::fire` commits goal progress and sends its congratulation
    message *before* the guarded write, so in the rare case the guard rejects, those
    side effects have already fired. Closing that needs the dispatch moved inside the
    transaction — a larger restructure, left as a follow-up. Also unchanged: the
    pre-existing `.sqlx` drift on `main` (`895e6b8`/`fc6caa8e`) recorded in the
    [CC-5 residual](_cross-cutting.md#cc-5), so `cargo sqlx prepare --check` still fails
    for that reason alone.
  - **This was the last open CC-9 site** — [CC-9](_cross-cutting.md#cc-9) is now
    closable as bookkeeping.
- **Fix (2026-07-19, `daily`):** `DailyManager::save` (whole-row absolute upsert)
  replaced with `DailyManager::claim_daily`, a single guarded atomic upsert —
  `INSERT … ON CONFLICT DO UPDATE SET coins = gambling.coins + $2, daily = today
  WHERE gambling.daily <> today`. `rows_affected == 0` ⇒ already claimed today, so
  the command now also rejects a same-tick double-submit (previously only the
  in-memory date read guarded it). Closes both the lost-update and the residual
  double-credit window for `daily`.
- **`work` deferred** _(superseded — see the 2026-07-25 fix above)_**:** the `work` path interleaves `Dispatch::fire` (which mutates
  the row mid-flow), stamina bookkeeping (`verify_work`/`done_work`), gem rolls, and
  `mine_activity` before a single absolute `save`. A correct atomic conversion needs
  per-field deltas **and** a guarded stamina decrement (its own floor race, cf.
  DS-5) — i.e. the [CC-1](_cross-cutting.md#cc-1) concrete-`PgPool` refactor of the
  module, not a surgical patch. Left as-is to avoid a subtle regression in the most
  complex module; fold it in with the CC-1 migration.
- **Where:** `bot-modules/gambling/src/commands/daily.rs:114-129` and
  `commands/work.rs:~160-176` — `row = *_row()` → mutate `coins`/`gems` in memory →
  `Manager::save(pool, &row)` (whole-row absolute upsert).
- **What:** Same shape as [CC-9](_cross-cutting.md#cc-9), recorded here so the
  instance list is complete. The once-per-day date guard (`daily.rs:121`) prevents
  *double-credit*, but the absolute `save` still clobbers any coin/gem mutation that
  another command commits between this command's read and its `save`.
- **Failure scenario:** user has 1 000 coins, runs `/daily` (reads 1 000), and
  before its `save` lands a bet/`/work`/gift atomically credits +500 (→1 500). The
  daily `save` writes `coins = 1000 + amount`, silently erasing the +500.
- **Confidence:** confirmed-logic (absolute `save`), plausible-interleave.
- **Suggested fix:** fold into the CC-9 remediation — atomic
  `UPDATE … SET coins = coins + $amount, daily = $today WHERE daily <> $today`,
  assert `rows_affected == 1`.

### DS-8. Stamina cron `UPDATE` has no `WHERE` → full-table rewrite every 10 min → deadlocks with gameplay writes (+ slow statement, bloat)  ·  Pass 2 (concurrency) + Pass 6 (resource)  ·  high
- **Status:** `complete — 37bcf343`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot/src/bindings/gambling/stamina.rs:12-19`
  (`UPDATE gambling SET stamina = LEAST(stamina + 1, $1)` — **no `WHERE`**),
  scheduled `0 */10 * * * * *` (every 10 min) at
  `bot-modules/gambling/src/stamina.rs:16`.
- **What:** The regen tick locks and rewrites **every row** in `gambling`, the
  hottest table in the workspace — including the majority already at
  `MAX_STAMINA = 3`, where `LEAST` makes the value a no-op but Postgres still writes
  a new tuple and takes a row lock. The cron runs on the single cron loop
  (`bot/src/cron.rs:72-78` awaits `join_all`, so no self-overlap), but user
  gameplay runs on independent interaction tasks. A full-table `UPDATE` locks rows
  in **scan/physical order**; concurrent multi-row gameplay txns (`/send`'s two-row
  transfer, `/daily`, bets) lock rows in **user_id order** → lock-order inversion.
- **Failure scenario (production-confirmed, 2026-07-17):**
  1. *Deadlock* — `stamina cron update failed | error=Database(PgDatabaseError …
     code: "40P01", message: "deadlock detected" … while updating tuple (27,139) in
     relation "gambling")`. The stamina UPDATE and a concurrent gameplay txn each
     hold a row the other wants; Postgres aborts one. When the **cron** is the
     victim, its whole UPDATE rolls back → **no user regenerates stamina that tick**
     (all-or-nothing, one statement).
  2. *Slow statement / churn* — `slow statement … UPDATE gambling SET stamina =
     LEAST(stamina + 1, $1) … rows_affected=0 elapsed=1.003s`. 0 rows changed yet
     >1 s elapsed = the statement spent the time blocked on locks before being
     rolled back (the deadlock victim), and the WHERE-less rewrite bloats the table
     (a dead tuple per user per tick, most of them no-ops).
- **Why it matters:** intermittent loss of the stamina-regen tick (gameplay
  resource users depend on), recurring deadlock error noise, table bloat/VACUUM
  pressure on the busiest table, and added contention that widens the CC-9 race
  windows on the same rows.
- **Confidence:** confirmed (WHERE-less SQL read directly; both symptoms present in
  production logs; single-loop cron model rules out self-overlap, isolating the
  cause to cron-vs-gameplay lock inversion).
- **Suggested fix:** add `WHERE stamina < $1` so the tick touches only the small
  minority of rows that actually need regen — this cuts the lock footprint, the
  bloat, and the deadlock probability by orders of magnitude. For full robustness
  also wrap the cron write in a `40P01` retry (and/or `SET LOCAL lock_timeout`), and
  consider ordering the update (`… WHERE stamina < $1`) so it and gameplay agree on
  lock order.

### DS-9. `/shop buy` whole-row absolute overwrite → double-submit buys two items for one charge  ·  Pass 8 (CC-9 sweep)  ·  med-high
- **Status:** `complete — 80859b63`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/gambling/src/commands/shop/buy.rs:46-151` and
  `bot-modules/gambling/src/common/shop/mod.rs:71-152`
  (`ShopManager::buy_save` — `coins = EXCLUDED.coins, gems = EXCLUDED.gems` plus
  twelve absolute `gambling_mine` columns; `ShopManager::save_inventory` —
  `quantity = EXCLUDED.quantity`).
- **What:** A textbook [CC-9](_cross-cutting.md) site, and the largest one left
  after DS-1…DS-8: `buy` reads the whole `ShopRow` (coins, gems, the three
  crafted currencies, nine mine columns), deducts the cost **in memory**, awaits
  `Dispatch::fire` (which can credit goal rewards), then writes the resulting
  **absolute** values back. There is no `GameCache::check_and_set` guard on this
  path, unlike the wager games.
- **Failure scenario:**
  1. User holds 10 000 coins and double-submits `/shop buy` for a 4 000-coin
     item. Discord dispatches each interaction on its own tokio task
     (`bot/src/handler/interaction/mod.rs:168`); both read `coins = 10 000`.
  2. Both compute `10 000 - 4 000` and both write the **absolute** 6 000.
  3. The inventory upsert is `quantity = EXCLUDED.quantity` from each task's own
     in-memory count, so the item lands twice by the same lost-update — or once,
     depending on ordering. Either way **two items were delivered and one was
     charged**: 4 000 coins minted.
  4. Any concurrent credit (a `/gift`, a `/work` payout, a goal reward) landing
     in that window is erased by the absolute write.
  5. Mine purchases lose the symmetric way: two concurrent `miner` buys both
     pass the app-layer `max_values` check against the same stale snapshot, so
     the pair can land **above** the capacity cap, or one increment is dropped
     while both are charged.
- **Additional gap:** `save_inventory` committed on the pool at
  `buy.rs:129` — *before*, and outside, `buy_save`'s transaction. An error
  between the two granted the items with no debit at all.
- **Confidence:** confirmed-logic, plausible-interleave (same class as DS-2/DS-7,
  which were confirmed).
- **Fix (2026-07-28):** Replaced the read-modify-write with `ShopDelta` (the net
  change, taken *after* the dispatch so goal rewards ride along) plus
  `ShopManager::commit_purchase`, one transaction of atomic guarded writes:
  - `UPDATE gambling SET coins = coins + $2, gems = gems + $3 WHERE user_id = $1
    AND ($2 = 0 OR coins + $2 >= 0) AND ($3 = 0 OR gems + $3 >= 0)`.
  - `UPDATE gambling_mine SET <12 columns> = <column> + $n` with a per-column
    guard `($n = 0 OR col + $n BETWEEN 0 AND <ratio> * (<sibling> + $m + 1))`.
    The ceilings are evaluated against the **live** row, so a cap check that
    passed on a stale snapshot cannot be used to exceed the cap. The ratios are
    bound from the `MaxValues` trait fns rather than duplicated as SQL literals.
  - Inventory folded into the same transaction as
    `quantity = gambling_inventory.quantity + $3 RETURNING quantity`, closing the
    non-atomic gap above.
  - The `$n = 0 OR …` escape means a column that is not part of this purchase
    can never block it — an existing row already above a cap (or a legacy
    negative balance) still transacts on unrelated columns.
  - Guard rejection surfaces as the new `GamblingError::PurchaseConflict`
    ("your balance changed, try again") rather than a 500 or a silent overdraw.
    The app-layer pre-checks are kept for their precise user-facing messages;
    SQL is the authority for correctness.
  - `buy_save` and `save_inventory` are **deleted** — the absolute-write helpers
    are gone, not merely bypassed, so the pattern cannot regress here.
- **Verification:** `bot-modules/gambling/tests/shop_buy.rs` (8 tests,
  fails-before: `ShopDelta` did not exist). Gate: `cargo +nightly clippy
  --workspace --all-targets -D warnings` clean with no new `#[allow]`/`#[expect]`;
  `cargo test` green; `.sqlx` regenerated with `--all-features`.
- **Residual / follow-ups:**
  - **`/shop sell` is the same class and is NOT fixed here** — `sell_save`
    (`common/shop/mod.rs`) still writes `coins = EXCLUDED.coins` and an absolute
    `gambling_inventory.quantity` from a snapshot read at `sell.rs:74`. Recorded
    as **DS-10** below; kept out of this task to keep the diff reviewable.
  - `dig`, `craft`, `prestige` and `game_row` retain absolute `EXCLUDED` writes
    (see the CC-9 umbrella); each is its own task.

### DS-10. `/shop sell` is the same absolute-overwrite site as DS-9  ·  Pass 8 (CC-9 sweep)  ·  med
- **Status:** `complete — f7280e37`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/gambling/src/commands/shop/sell.rs:74-91` and
  `ShopManager::sell_save` (`bot-modules/gambling/src/common/shop/mod.rs`).
- **What:** `sell` reads `SellRow` (coins + the item's inventory quantity),
  credits the payment and decrements the quantity in memory, then persists both
  absolutely (`coins = EXCLUDED.coins`, `UPDATE gambling_inventory SET quantity = $1`).
- **Failure scenario:** double-submitting a sale of the last N of an item has
  both invocations read `quantity = N`, both write `quantity = 0`, and both
  credit the payment via an absolute coin write — so the payout that lands last
  wins, but the **item is consumed once**. Mirrored against a concurrent credit,
  the sale erases it. Same shape as DS-9, opposite direction.
- **Suggested fix:** the DS-9 pattern — `coins = coins + $payment` and
  `UPDATE gambling_inventory SET quantity = quantity - $n WHERE quantity >= $n`
  in one transaction, `RETURNING` the post-image; delete `sell_save`.
- **Fix (2026-07-28):** Applied the DS-9 pattern. `SaleDelta` (payment credited,
  units removed — the sales-tax arithmetic moved out of the command into
  `SaleDelta::new`) plus `ShopManager::commit_sale`, one transaction of atomic
  guarded writes:
  - `UPDATE gambling SET coins = coins + $2 WHERE user_id = $1 RETURNING coins`.
  - `UPDATE gambling_inventory SET quantity = quantity - $3 WHERE user_id = $1
    AND item_id = $2 AND quantity >= $3 RETURNING quantity` — the guard is
    evaluated against the **live** row, so a stock check that passed on a stale
    snapshot cannot oversell.
  - The row is `DELETE`d only when the *returned* post-image is 0, inside the
    same transaction (previously the delete was keyed off the in-memory value).
  - `gambling` is locked **before** `gambling_inventory`, matching
    `commit_purchase`'s order, so a concurrent buy and sell of the same item
    cannot deadlock on inverted lock order.
  - Guard rejection surfaces as `GamblingError::TransactionConflict` (the DS-9
    `PurchaseConflict` variant, renamed and its message made direction-neutral
    so it reads correctly on both the buy and sell paths).
  - `sell_save` and `SellRow` are **deleted**. The pre-check read is now
    `sell_quantity` — a single-table `SELECT quantity`, replacing the `LEFT JOIN`
    that only existed to fetch the coins the command no longer mutates in
    memory. Its only job is choosing between the `ItemNotInInventory` and
    `InsufficientItemQuantity` messages; SQL is the authority for correctness.
- **Verification:** `bot-modules/gambling/tests/shop_sell.rs` (7 tests,
  fails-before: `unresolved import gambling::SaleDelta`). Gate: `cargo +nightly
  clippy --workspace --all-targets -- -D warnings` clean with no new
  `#[allow]`/`#[expect]`; `cargo test --workspace` green; `.sqlx` regenerated
  with `--all-features` (4 entries removed, 4 added, no unrelated churn) and
  re-verified with `SQLX_OFFLINE=true`.
- **Residual / follow-ups:**
  - `dig`, `craft`, `prestige` and `game_row` retain absolute `EXCLUDED` writes
    (see the [CC-9](_cross-cutting.md) umbrella); each is its own task. With
    DS-10 closed, `common/shop/mod.rs` itself has no absolute-write helpers left.
  - Neither `/shop buy` nor `/shop sell` is gated by `GameCache::check_and_set`,
    so a double-submit still costs two round-trips — it is now merely *correct*
    rather than exploitable. An idempotency gate is a separate concern.

### DS-11. `/dig` writes both `gambling` and `gambling_mine` absolutely  ·  Pass 8 (CC-9 sweep)  ·  med-high
- **Status:** `complete — 1904945b`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `DigManager::save` (`bot-modules/gambling/src/commands/dig.rs:84-140`),
  called from `Commands::dig` (`dig.rs:319`).
- **What:** `/dig` reads coins/gems/stamina plus the ten `gambling_mine` columns,
  mutates them in memory across two awaits (`Dispatch::fire`, then the mine
  payout and `done_work()`), and persists both rows absolutely —
  `coins = EXCLUDED.coins, gems = EXCLUDED.gems, stamina = EXCLUDED.stamina` and
  `coal = EXCLUDED.coal, … prestige = EXCLUDED.prestige`. There is no
  `WHERE stamina > 0` guard, so SQL never re-checks the stamina the command
  approved from a snapshot.
- **Failure scenario:** double-submitting `/dig` has both invocations read
  `stamina = N`; both write `N - 1`, so **one dig is free**, and the second
  absolute write clobbers the first's ore gains and mine payout — one dig's whole
  yield vanishes. At `stamina = 1` both digs pass the app-layer `verify_work()`
  and land the row at 0. Against a concurrent `/work`, `/shop buy` or a wager
  payout, the dig's absolute coin write erases that transaction outright.
- **Suggested fix:** the DS-7 (`/work`) pattern — a `DigDelta` committed by
  `DigManager::commit_dig` in one transaction: atomic `coins = gambling.coins +
  $n`, `stamina = gambling.stamina - 1 WHERE gambling.stamina > 0`, and
  `coal = gambling_mine.coal + $n` per ore column; delete `save`. The hourly mine
  accrual must **not** simply become additive — see the payout note below.
- **Fix (2026-07-28):** `DigDelta` (the change a dig made to the row it read)
  plus `DigManager::commit_dig`, one transaction of atomic guarded writes:
  - `INSERT … ON CONFLICT DO UPDATE SET coins = gambling.coins + $2, gems =
    gambling.gems + $3, stamina = gambling.stamina - 1 WHERE gambling.stamina > 0
    RETURNING coins, gems, stamina` — the stamina guard is evaluated against the
    **live** row, so two digs racing on the last point cannot both succeed;
    rejection surfaces as the existing `out_of_stamina()` error, as in `/work`.
  - `gambling_mine` ore columns become `coal = gambling_mine.coal + $2, …` —
    each dig keeps its own roll. `miners` and `prestige` are dropped from the
    write entirely: `/dig` only ever read them, so writing them back could only
    ever clobber a concurrent `/shop buy`.
  - `gambling` is locked **before** `gambling_mine`, matching `commit_purchase` /
    `commit_sale` / `commit_work`, so no inverted lock order.
  - **The hourly accrual is deliberately not additive.** It is time-based, so
    crediting it as an increment would let two digs in the same tick each collect
    the same hours — trading the lost update for a mint. It travels as
    `MinePayout { coins, since, collected_at }` and `mine_activity` advances via
    a compare-and-swap (`CASE WHEN gambling_mine.mine_activity = $10 THEN
    EXCLUDED.mine_activity ELSE gambling_mine.mine_activity END`, with
    `RETURNING mine_activity = $9`). The payout is credited only by the dig that
    wins that swap; a loser still gets its ore and still spends its stamina, and
    reports `0` collected. `save` is **deleted**.
- **Verification:** `bot-modules/gambling/tests/dig.rs` (6 tests, fails-before:
  `unresolved import gambling::DigDelta`). Gate: `cargo +nightly clippy
  --workspace --all-targets -- -D warnings` clean with no new
  `#[allow]`/`#[expect]` (the one `#[expect(trivial_casts)]` is the pre-existing
  sqlx bind-override on `mine_activity`, carried over from the deleted `save`);
  `cargo test --workspace` green.
- **Residual / follow-ups:**
  - **`/work` collects the same accrual without a CAS** — `commit_work`
    (`commands/work.rs:175`) still writes `mine_activity = EXCLUDED.mine_activity`
    unconditionally and folds `mine_amount` into `WorkDelta`, so a `/work` racing
    a `/dig` can still pay the accrual twice. `/dig` now refuses its half of that
    race; closing it fully needs the same `MinePayout` treatment in `/work`.
    Recorded as **DS-12** below.
  - `craft`, `prestige` and `game_row` retain absolute `EXCLUDED` writes (see the
    [CC-9](_cross-cutting.md) umbrella); each is its own task.
  - `/dig` is not gated by `GameCache::check_and_set`, so a double-submit still
    costs two round-trips — it is now merely *correct* rather than exploitable.

### DS-12. `/work` collects the hourly mine accrual without a compare-and-swap  ·  Pass 8 (CC-9 sweep)  ·  low-med
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `WorkManager::commit_work`
  (`bot-modules/gambling/src/commands/work.rs:140-189`), called from
  `Commands::work` (`work.rs:253`).
- **What:** the DS-7 fix made `/work`'s coin/gem writes atomic, but the hourly
  mine payout (`row.mine_amount()`) is folded into `WorkDelta.coins` and
  `mine_activity` is stamped absolutely (`mine_activity = EXCLUDED.mine_activity`),
  so the accrual is credited unconditionally.
- **Failure scenario:** two `/work`s in the same tick both read `mine_activity`
  at `T0`, both compute the same accrued hours, and both credit them — the same
  mine output is paid twice for two stamina. The pre-DS-7 absolute write masked
  this by dropping one of the two payouts; the atomic increment does not.
  Mirrored against `/dig`, whose accrual is now CAS-guarded (DS-11), `/work`
  still wins the double payment.
- **Suggested fix:** the DS-11 pattern — pull `mine_amount` out of `WorkDelta`
  into a `MinePayout`, advance `mine_activity` only when it still equals the
  stamp the payout was computed from, and credit the payout only on that swap.
  `MinePayout` already exists (`commands/dig.rs`); this is mostly a move.
- **Fix (2026-07-28):** as suggested — the DS-11 shape, applied to `/work`.
  - `MinePayout` moved from `commands/dig.rs` to `models/mod.rs`, next to the
    `MineAmount` trait that computes the accrual it carries, and re-exported from
    there. Both collectors now share one type instead of `/work` growing a copy;
    the doc comment on it records *why* a time-based accrual must not become an
    atomic increment, so the next site to touch it inherits the reasoning.
  - `Commands::work` credits only `base_amount` (`rand::random_range(100..=500)`)
    to the in-memory row, so `WorkDelta` — taken after `Dispatch::fire` — carries
    the shift's own earnings and goal rewards but **not** the accrual. The
    accrual travels as `MinePayout::new(row.mine_amount()?, row.mine_activity())`.
  - `commit_work` keeps its existing guarded stamina/coins upsert, then advances
    the watermark by compare-and-swap — `mine_activity = CASE WHEN
    gambling_mine.mine_activity = $3 THEN EXCLUDED.mine_activity ELSE
    gambling_mine.mine_activity END`, with `RETURNING mine_activity = $2 AS
    "claimed!"` — and credits `payout.coins` in a second atomic `coins = coins +
    $2` **only** when that swap was won, all inside the same transaction. Lock
    order (`gambling` before `gambling_mine`) is unchanged, so it still matches
    `commit_purchase` / `commit_sale` / `commit_dig`.
  - `WorkCommit` gains `payout: i64` — what this shift *actually* collected. The
    embed's "Collected N coins" line is now `base_amount + committed.payout`
    rather than the optimistically computed total, so a shift that loses the swap
    reports what it was really paid instead of claiming coins it never received.
  - The unconditional `mine_activity = EXCLUDED.mine_activity` write is gone.
- **Verification:** `bot-modules/gambling/tests/work.rs` grows four DS-12 tests
  alongside the existing four DS-7 ones (8 total, all green). Fails-before:
  `error[E0560]: struct WorkCommit has no field named payout` — the pre-fix
  commit had no way to report a payout it might not have made. Gate:
  `cargo +nightly clippy --workspace --all-targets -- -D warnings` clean, no new
  `#[allow]`/`#[expect]` (the one `#[expect(trivial_casts)]` is the pre-existing
  sqlx bind-override on `mine_activity`); `cargo test --workspace` green, 0
  failures; `cargo sqlx prepare --workspace -- --all-features` regenerated
  against an empty freshly-migrated database.
- **Residual / follow-ups:**
  - **`/work` and `/dig` now interlock correctly**, closing the CC-9 time-accrual
    sub-class: whichever of the two lands first takes the window, the other is
    paid `0` for it and still earns its own base/ore.
  - `craft`, `prestige` and `game_row` retain absolute `EXCLUDED` writes (see the
    [CC-9](_cross-cutting.md) umbrella); each still needs its own `DS-#`.
  - `/work` is still not gated by `GameCache::check_and_set`, so a double-submit
    costs two round-trips and two stamina — correct, but not idempotent.
  - The regenerated cache also corrects an **unrelated** pre-existing entry —
    the `lfg_posts LEFT JOIN lfg_user_settings` nullability in
    `bot/src/bindings/lfg/mod.rs`, which had been recorded from a populated dev
    DB (`[T,T,T,T,T,F]`) and is now the empty-DB truth (`[F,F,F,F,F,T]`). It
    rides along in this diff rather than being split out.
