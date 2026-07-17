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
- **Where:** `src/models/*`, `src/commands/*`, `src/games/*`, `src/common/*`,
  `src/components/*` — nearly every file threads `<Db: Database>` / `Pool<Db>`;
  concrete impls live in `bot/src/bindings/gambling/*`.
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
