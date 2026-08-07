# Audit: bot

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

The binary crate (~10k LOC, 99 files) that wires every module into Serenity:
`bindings/*` (the `ModuleComponent`/`ModuleModal` impls + the concrete `…Table`
manager impls), the `moderation` feature (which lives here, not in a crate), the
event `handler`, and the `registry`/`cron` scaffolding. Health is decent; the
issues are the concentration of `#[expect]` escape-hatches, an inline test
module, the concrete SQL impls for the CC-1 modules living here (a *symptom* of
CC-1, not a bug in `bot`), and the structural inability to host integration
tests (no lib target).

## Findings

### 1. `#[expect]` cluster across bindings  ·  #7  ·  med
- **Where:** `src/handler/mod.rs:121`,
  `src/bindings/gambling/{goals,daily,dig,work}.rs`,
  `src/bindings/lfg/mod.rs:159,189`, `src/bindings/levels/mod.rs:105`,
  `src/bindings/temp_voice/mod.rs:132`.
  (The former `src/bindings/moderation/infraction.rs:210`
  `#[allow(clippy::too_many_arguments)]` is **gone** as of the DS-2 revival —
  the infraction fields were bundled into a `Case` struct, exactly the refactor
  suggested below.)
- **What / Why / Fix:** See [CC-3](_cross-cutting.md#cc-3). The
  `too_many_arguments` on the infraction writer was the clearest refactor target
  (bundle the infraction fields into a struct) — now done.

### 2. Inline `#[cfg(test)]` module  ·  #6  ·  med
- **Where:** `src/registry/dispatch_map.rs:103`.
- **What:** Inline test module (CC-2), but `bot` has **no lib target**, so it
  can't host a `tests/` integration file without either adding a lib target or
  moving the tested routing logic into a lib crate (e.g. `zayden-core`).
- **Why it matters:** The dispatch/overlap logic is worth testing but currently
  can only be tested inline.
- **Suggested fix:** Extract the pure dispatch-map logic into a lib crate
  (`zayden-core`) and test it there, or add a `[lib]` to `bot`. See
  [CC-2](_cross-cutting.md#cc-2).

### 3. Concrete SQL for CC-1 modules lives here  ·  #1  ·  med (tracked in CC-1)
- **Status:** `complete — with CC-1`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Reconciled (2026-07-29):** Closed as a sub-item of
  [CC-1](_cross-cutting.md#cc-1), whose per-module migrations moved each
  module's SQL home. Verified on `bf0d90ff`: **no
  `impl XxxManager<Postgres> for XxxTable` bodies remain** anywhere in
  `bot/src/bindings/`, and the only `query!`/`query_as!`/`query_scalar!` call
  sites left under `bindings/` are in `moderation/{mod,rules,infraction}.rs`.
- **Residual (deliberate, not a CC-1 leftover):** the `moderation` SQL stays in
  `bot/`. `moderation` is **not a crate** — it is a set of bindings under
  `bot/src/bindings/moderation/` ([`README.md:130`](README.md)) — so there is no
  module crate for its SQL to move into, and it never used a DB-generic manager
  trait. Extracting it into its own crate would be a new finding, not this one.
- **Where:** `src/bindings/{gambling,levels,gold_star,family,…}/…` —
  `impl XxxManager<Postgres> for XxxTable` bodies.
- **What:** Because the manager traits are DB-generic, each module's SQL is
  implemented here in `bot/` rather than in the module crate. This is the
  scattering described in [CC-1](_cross-cutting.md#cc-1) — resolved when those
  modules go concrete and their SQL moves home.
- **Note:** `gold_star.rs` additionally uses runtime SQL — see
  [CC-5](_cross-cutting.md#cc-5) / [gold-star.md](gold-star.md).

### 4. Handful of `unwrap()`/`expect()` + a correctness TODO  ·  #3 / #2  ·  low → **med in practice**
- **Status:** `complete — 7e75cc79`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Sha corrected 2026-08-07.** This marker read `complete — 12f6b01e`, which is
  not on `main`: the commit was amended (the audit-doc hunk grew) and the landed
  commit is **`7e75cc79`**, same message, identical `src/` content — `git show
  --stat` on both lists the same 11 paths and the same code deltas, differing
  only in `bot.md`'s line count. Nothing about the fix changed; the record
  pointed at a dangling object. Recorded because it is a *new* way for the
  2026-07-29 "reconcile against the tree" lesson to bite — a sha can go stale
  without the claim it supports being wrong.
- **Committed 2026-08-06 as `7e75cc79`** ("Refactored pending_jobs") during the
  review round, so the code landed before this note was finished. Verified
  against the tree, not the record: the commit carries the `prune_exhausted`
  split (`bot/src/cron.rs:93-96` takes the write guard and prunes), all **10**
  tests in `zayden-core/tests/cron.rs`, and the `bot/src/reset.rs` deletion.
- **Fix (2026-08-06).** The fifth untagged "confirm X" item, and the fifth to
  **fail its confirmation** — on both halves, in opposite directions. Neither
  half was what the finding described.
- **Half 1 — the 6 `unwrap()`s were not live code.** All six sit in
  `bot/src/reset.rs`, a file that is **not `mod`-declared** in `main.rs`
  (`bindings`, `cron`, `error`, `handler`, `registry`, `sqlx_lib`, `state`,
  `webhook_logger` — no `reset`) and is referenced from nowhere in the
  workspace. It has never compiled, so there was nothing to "verify the
  `unwrap()`s are on genuinely-infallible values" *about*: rustc never type-checked
  them and `-D warnings` never saw the file. It is the same orphaned-tree class as
  **DS-2** below (`bindings/moderation`, `bot.md:213`), which is why the sweep's
  per-crate lens missed both.
  What it actually contained was a hand-run dev DB-reset script whose one
  uncommented statement is `TRUNCATE TABLE gambling_effects RESTART IDENTITY`
  — destructive economy SQL, unreviewed because unbuildable. **Deleted.** The
  2026-07-29 reserved-consts policy does not protect it: that covers reserved
  *catalogue items* (shop items), not an orphaned script.
- **Half 2 — the cron TODO was a real defect, not a redundancy.** The TODO
  guessed that `t > now` and `includes(t)` were both redundant. Half right:
  - `t > now` **is** redundant. `Schedule::upcoming(tz)` is
    `after(Zoned::now())`, and the iterator yields times strictly after its start
    point, which is never earlier than the `now` captured a few lines above.
  - `includes(t)` is **not**. `jiff_cron 0.3.0`'s `next_after` has a fast path
    (round the start time up to the next whole second; return it if every field
    matches) that checks seconds, minutes, hours, day-of-month, month and year —
    **but not day-of-week** (`schedule.rs:218-241` vs. the `days_of_week` check at
    `:300-306`, which only guards the slow path). So a `Fri`-only schedule sampled
    at Thursday 16:59:59.5 proposes **Thursday 17:00:00**, and the same schedule's
    `includes` correctly rejects it. Reproduced as a test, not reasoned about:
    `jiff_cron_after_can_propose_a_wrong_weekday`.
- **Why that mattered:** the old code fed that rejection into
  `data.jobs_mut().retain(...)` — so a job whose schedule *momentarily*
  mispredicted was **permanently deleted from `BotState`** for the life of the
  process, silently, with no log line. Four registered jobs are weekday-restricted
  and therefore exposed: gambling `lotto` and `higherlower`
  (`"0 0 17 * * Fri *"`), destiny2 `endgame_analysis_sheet_weekly`
  (`"0 0 0 * * Mon *"`), marathon `marathon_schedule_announce`
  (`"0 0 17,18 * * Sun,Thu *"`). A lost `lotto` job means the draw never runs
  again until restart — the same silent-cron class as the seventh pass's
  prod findings, which is why this is recorded as **med in practice** against the
  audit's `low`.
- **Fix shape — the two predicates are now separate functions, which is the whole
  point.** The old `retain` conflated *"this candidate is invalid"* with *"this
  schedule is over"*, and that conflation is the defect. Split in `zayden-core`:
  - `earliest_pending(&[CronJob], &Zoned)` selects, and takes a **shared slice**,
    so removing a job is not expressible there at all. Its `includes` guard is
    kept and **strengthened** from `.next().filter(includes)` to
    `.find(includes)`, so a wrong-weekday proposal makes the job resolve to its
    real next occurrence (Friday) instead of vanishing from the tick. At most one
    candidate is ever skipped — the fast path only fires for a sub-second start
    time, and every value the iterator yields after that is whole-second.
  - `prune_exhausted(&mut Vec<CronJob>, &Zoned)` removes, under the *only* safe
    predicate: `.next().is_none()`, meaning the schedule has no occurrence left
    at all. Deliberately **not** `next_run` — an `includes` rejection must never
    remove anything.
- **The prune is load-bearing, and dropping it was a regression I nearly shipped.**
  An intermediate version of this fix deleted the `retain` outright and moved
  `pending_jobs` to a **read** guard. That is wrong: LFG reminders are *one-shot*
  jobs — `lfg::cron::create_reminders` builds four year-pinned schedules per post
  (`"0 {min} {hour} {day} {month} * {year}"`, `reminders.rs:28-106`) — and once
  fired their iterator is empty forever. Verified, not assumed:
  `Schedule::from_str("0 30 14 3 6 * 2024").after(2026-08-06).next()` is `None`.
  `jobs_mut()` has exactly two callers in the workspace, both in
  `reminders.rs:112-113`, and the `retain` there only evicts the post's **own**
  id before re-adding it — so nothing collects a *fired* reminder. Without the
  prune every LFG post leaks four `CronJob`s for the process lifetime, and each
  dead job costs a **failed ordinal search** (the expensive `next_after` path,
  which walks to the year ceiling before returning `None`) on every tick. Caught
  by the reviewer, not by the gate — no test covered the lifecycle because none
  existed.
- **So `pending_jobs` still takes a write guard**, and the "read guard" win the
  intermediate version claimed is withdrawn. Two corrections to that claim while
  it is being withdrawn: the guard is **not** taken every ~5 s — `pending_jobs`
  runs once per scheduling *cycle*, and a cycle is the gap to the next due job
  (~2 min with the current set, whose most frequent member is palworld's
  `save_refresh`; the 5 s is only a floor on the tail sleep). And it is held for
  microseconds of synchronous work. The contention was never the problem worth
  trading correctness for.
- **Why `bot` could host no test, and what changed:** `bot` has no `[lib]` target
  (the CC-6 / DS-3 constraint). Rather than record another "structurally
  infeasible", the pure selection logic moved into `zayden-core` — the
  [CC-2](_cross-cutting.md#cc-2) `DispatchMap` precedent, and llamad2's "reaching
  the logic may need a small extraction". `bot/src/cron.rs::pending_jobs` is now
  6 lines of ctx/lock plumbing over a tested function.
- **Verification.** `bot-modules/zayden-core/tests/cron.rs`, 10 tests. The logic was
  extracted and fixed in one step so no test can fail against the literal
  pre-image; the CC-6 guard-removal matrix stands in, and **both** mutations are
  caught by `weekday_restricted_job_resolves_past_a_wrong_weekday_candidate`:
  `.find(includes)` → `.next()` (no guard) resolves to Thursday 17:00;
  `.find(includes)` → `.next().filter(includes)` (the exact old predicate) leaves
  the job unscheduled (0 pending). Wrong-day *execution* is prevented by that
  guard; wrong-day *deletion* is prevented **structurally**, by
  `earliest_pending`'s `&[CronJob]` signature rather than by an assertion — stated
  plainly rather than claimed as test coverage. The one-shot lifecycle the
  reviewer surfaced has three of its own: `a_fired_one_shot_is_not_pending`,
  `fired_one_shots_are_pruned` (fired LFG jobs go, recurring and future ones
  stay), and `a_recurring_job_survives_the_wrong_weekday_window` — which pins the
  split predicate directly by pruning all four weekday-restricted schedules in
  the wrong-weekday window and asserting none is removed.
- **Gates (`SQLX_OFFLINE=true`):** `cargo +nightly clippy --workspace
  --all-targets -- -D warnings` exit 0 and `.bacon-locations` empty;
  `cargo test --workspace --no-fail-fast` **640 passed / 0 failed**, exit 0
  (630 baseline = CC-10's 615 + temp-voice #4's 15, plus these 10);
  `cargo machete` clean; `cargo +nightly check -p dashboard --features ssr`
  exit 0; `cargo +nightly fmt` clean. No new `#[allow]`/`#[expect]`. No SQL
  touched, so no `.sqlx` regen. `Cargo.toml` delta: `jiff` added to `zayden-core`
  (`Zoned` is now in its public signature) — machete re-run for it.
- **Residual / follow-ups:** the `includes` guard works around an **upstream**
  bug; the real fix belongs in `jiff_cron`. `jiff_cron_after_can_propose_a_wrong_weekday`
  asserts the upstream behaviour deliberately, so a future `jiff_cron` bump that
  fixes it turns that test red rather than letting the workaround rot silently —
  do not delete it without re-reading `next_after`. Not addressed here: the
  scheduler still runs only the **strictly-earliest** tied jobs per tick and
  `join_all`s them onto one task (the palworld #2 observation); that is a separate
  finding if it is ever worth one.
- **Also left, and arguably the more interesting residual:** a fired LFG reminder
  is only collected on the **next scheduling tick**, and a *deleted* post's
  reminders are never collected early at all — `reminder()` handles
  `sqlx::Error::RowNotFound` by returning (`reminders.rs:120`), so the job still
  fires, does nothing, and waits for its year to pass before `prune_exhausted`
  can see it. That is bounded and harmless, but it means the registry's size
  tracks *scheduled* posts rather than *live* ones. Worth its own finding only if
  LFG volume ever makes it matter.
- **Lesson for the workflow:** a self-flagged `TODO` is a **hypothesis, not a
  finding**. This one named the right line and got the conclusion backwards, and
  the third deep-sweep pass had already traced the same scheduler "clean" — so
  two prior records agreed on the wrong answer. Reproducing the predicate against
  the library (nine lines, one `#[test]`) settled in a minute what neither read
  had. The corollary bit on the way out: **deleting the thing you proved wrong is
  not the same as deleting the thing it was doing.** The `retain` was both a
  broken guard *and* the registry's only garbage collector; removing it wholesale
  fixed the first and silently broke the second.
- **Where:** 6 `unwrap()`/`expect()` sites in `src/` (the only crate with a
  cluster); `src/cron.rs:93` — `// TODO(M9-correctness): verify retain predicate
  - upcoming().next()`.
- **What / Why:** Verify the `unwrap()`s are on genuinely-infallible values; the
  cron `retain` predicate has a self-flagged correctness question.
- **Suggested fix:** Audit the 6 sites individually; resolve the cron TODO
  (confirm the `upcoming().next()` retain logic drops fired jobs correctly).

## Clean
- #1 Architecture: `bindings/` per-module, `handler/`, `registry/`, `cron.rs`
  cleanly separated; `ModuleComponent`/`ModuleModal` routing consistent.
- #1 DB access: bindings use compile-time macros (except `gold_star.rs`, CC-5).
- #2 Moderation (`InfractionKind` `sqlx::Type`, `NO_REASON`, `LogFilter`) — the
  magic-string cleanup is done, and as of the DS-2 revival the tree is **live**
  (registered via `moderation::register`), no longer dead code.
- #4 Stringly typing: mostly typed; residual `custom_id.as_str()` routing in
  `bindings/gambling/{prestige,blackjack}.rs` (CC-7).

## Deep-sweep findings

_Deep sweep (sixth pass): 2026-07-17. Two new defects in the `bot` wiring layer
that no per-crate audit could see, because both live in the glue between a module
crate and its binding — exactly the blind spot CC-1/CC-9 describe. DS-3 was added
2026-07-22, split out of the DS-2 revival's residual note._

### DS-1. Level-up coin reward is a second transaction after XP is already committed → reward silently lost  ·  Pass 1 (silent failure) / SQL atomicity  ·  med
- **Status:** `complete — 82f308a2`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-19):** Pulled `levels::message_create` out of the `try_join!` and
  run it (plus the `add_coins` reward) **before** the fallible siblings
  (`Ai`/`support`/`llamad2`). Since the level is committed inside
  `message_create`, running it first makes the reward happen-before any sibling
  that can error, so a sibling failure can no longer short-circuit and drop an
  earned level-up reward. **Residual (documented in code):** the XP save and the
  reward are still two transactions, so a failure of the reward's own `commit`
  after XP is saved still drops it — closing that needs folding the reward into
  `message_create`'s transaction (a levels/gambling refactor, cf. CC-1).
- **Where:** `bot/src/handler/message_create.rs:35-56`.
- **What:** `levels::message_create` (inside the `tokio::try_join!` at :35)
  persists the new XP **and the incremented level** to its own row via
  `Manager::save` (autocommit) and returns `Some(level)`. The matching reward —
  `GamblingTable::add_coins(tx, author, level*1000)` — only runs *after* the whole
  `try_join!` resolves `Ok` (:44-55), in a **separate** transaction. `try_join!`
  short-circuits on the first sibling error, and the reward block is also gated
  behind its own `?`.
- **Failure scenario:** a user's message crosses a level threshold, so
  `levels::message_create` increments `level` (0→1) and commits it. In the *same*
  `try_join!`, a sibling future errors — `Ai::run` hits an OpenAI/network error, or
  `support(...)`/`llamad2` hits a transient DB/Discord error (all reachable; `Ai`
  does live network I/O). `try_join!` returns that `Err`, `?` returns from
  `message_create`, and the `if let Some(level)` reward block never executes → the
  **1 000-coin level-up reward is never credited**. On the next qualifying message
  `new_message()` sees `level == 1` already and only re-awards if XP crosses the
  *next* threshold, so the skipped reward is gone permanently. (Same loss if the
  reward's own `tx.commit()` fails after XP is saved — the two writes are not
  atomic regardless of siblings.)
- **Confidence:** confirmed (traced `new_message` mutates+persists `level` in
  `levels/src/sqlx_lib.rs:193-208`; reward is a post-join separate tx).
- **Suggested fix:** fold the level-up reward into the *same* statement/transaction
  that persists the level (e.g. return the reward delta from `message_create` and
  apply XP-save + `add_coins` in one tx, or credit inside `save`). At minimum move
  the reward *before* the fallible siblings, or make it idempotent/retryable.

### DS-2. Entire `bindings/moderation/` tree is orphaned → moderation is a dead feature (and 3 latent bugs hide in it)  ·  Pass 9 (drift) / #2  ·  med
- **Status:** `complete — fef9f933` (revived)            <!-- open | in-progress | in-review | complete | wontfix -->
- **Decision (2026-07-20):** **Revive**, not delete (owner's call). The tree is
  now a live feature on the current convention.
- **Fix (2026-07-20):** Rewrote the whole tree against the current API and wired
  it in. Specifically:
  - **Migrated** `Infraction`/`Logs`/`RulesCommand` from the obsolete
    `core::SlashCommand<Error, Postgres>` trait (which no longer exists — the
    cause of the "cannot compile" note) to `ModuleCommand` + `InvocationCtx` +
    `HandlerError`, using concrete `PgPool` (`cx.app.db`) — no DB-generic
    manager, so it sidesteps CC-1 entirely.
  - **Registered** it: added `pub mod moderation;` and
    `moderation::register(&mut builder);` to `bot/src/bindings/mod.rs`, and
    changed `register` to the `builder.add_command(..)` form. `/infraction`,
    `/logs`, `/rules` now dispatch (no command-name collision — verified).
  - **Dropped `chrono`:** the six-month recency window is now a SQL predicate
    (`created_at > now() - INTERVAL '6 months'`) instead of in-Rust `NaiveDateTime`
    math; timeouts use serenity's `Timestamp` (no new dep, so `cargo machete`
    N/A). Deleted the two empty placeholder files (`infraction_kind.rs`,
    `infraction_row.rs`).
  - **Fixed the 3 latent bugs:** (1) `mute()` now records `InfractionKind::Mute`
    (was `Ban`); (2) the reachable `unreachable!()` is gone — the escalation
    count is `clamp(1, 5)` and the `points` option has `min_int_value(1)`, and
    the match uses `..=1`/`_` arms so no panic path exists; (3) `ban()` applies
    the ban regardless of DMs — the notify DM is now best-effort (`let _ =`)
    instead of `?`-propagated, so a user who blocks server-member DMs can still
    be banned (all three actions now DM best-effort, resolving the ordering
    inconsistency).
  - **Removed the CC-3 `#[allow(too_many_arguments)]`:** the action helpers take
    a single `Case<'_>` struct (bundling ctx/pool/guild/target/moderator/points/
    reason). Also corrected this file's Finding #1 and Clean §#2, and CC-3's
    inventory, which described the tree as live/dead-code.
  - **New `.sqlx` entries:** the 2 `user_infractions` SELECTs + the `record`
    INSERT (plain queries, no LEFT JOIN → no nullability drift).
  - **Verification:** compiles live + offline; workspace clippy `-D warnings`
    clean (the `Case` bundling means **no new `#[allow]`**); 257 tests pass.
    No new tests — the commands are thin Discord-action wrappers with no
    pure-logic surface, and `bot` has no lib target to host integration tests
    (CC-2). **Residual:** the hardcoded College-Kings `CHANNEL_ID`/`MESSAGE_ID`
    in `/rules` are pre-existing magic values (own finding, not DS-2); `Kick`/
    `SoftBan` remain defined-but-unused enum variants (no command emits them).
- **Where:** `bot/src/bindings/moderation/*` — never `mod`-declared in
  `bot/src/bindings/mod.rs:6-22` (which lists every *other* binding). Nothing in
  `bot/src` references `moderation` outside that directory, and its
  `infraction.rs:5` `use core::{SlashCommand, parse_options}` names a
  `SlashCommand` trait that **does not exist anywhere in the workspace** — so the
  tree cannot compile and is excluded from the build.
- **What:** `moderation::register()` (`mod.rs:23`) is never called; `/infraction`,
  `/logs`, `/rules` are defined but **never registered or dispatched**. Moderation
  is an advertised-but-absent feature.
- **Why it matters / doc correction:** two baseline records describe this code as
  *live*: **CC-3** lists `bot/src/bindings/moderation/infraction.rs:210` among the
  23 active `#[allow]`/`#[expect]` sites, and this file's own **Clean §#2**
  ("Dead code (moderation): M2 landed `InfractionKind` … the magic-string cleanup
  is done") treats it as shipped. Both are describing dead, uncompiled code. If the
  project believes it has an infraction/mod-log system, it does not.
- **Latent bugs that would surface the moment it is wired (informational — currently unreachable):**
  1. `mute()` records the infraction with `InfractionKind::Ban` instead of `Mute`
     (`infraction.rs:234`) → every mute is logged in history as a ban.
  2. `unreachable!("Invalid infraction count")` (`infraction.rs:109`) is *reachable*:
     the `points` option (`infraction.rs:132-136`) has no `min_int_value`, so
     `points ≤ 0` makes `infraction_count = min(sum+points, 5) ≤ 0` (`:41,:65`) and
     the `match` falls through → panic.
  3. `ban()` DMs the target (`:295`) **before** `member.ban_with_reason` (`:297`),
     both behind `?`, so a user who blocks DMs from server members (`direct_message`
     → 403) **cannot be banned** via the command. `mute()`/`warn()` apply the action
     first, so the ordering is also internally inconsistent.
- **Confidence:** dead-feature confirmed (grep-verified no `mod`/`use`/`register`
  reference; missing `SlashCommand` trait). The 3 sub-bugs are latent — recorded so
  they are fixed *before* the tree is wired, not after.
- **Suggested fix:** decide the feature's fate. If wanted: give it a live home
  (a `moderation` crate using the `ModuleComponent`/`SlashCommand` convention, or
  wire the binding + fix the 3 bugs) and add it to `register`. If not: delete the
  tree and correct CC-3 and Clean §#2.

### DS-3. `/rules` is hardcoded single-guild (magic IDs + on-disk `messages/rules.md`) → unusable by any other guild  ·  Pass 9 (drift) / #2  ·  med
- **Status:** `complete — 2138829c`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-22):** `/rules` is now a guild-generic, DB-backed command group.
  New migration `0014_guild_rules` adds `guild_rules` (per-guild config:
  `channel_id`, nullable `message_id`, `title`/`description`/`colour` styling) and
  a child `guild_rule` (ordered `position`, `title`, `body`) with
  `ON DELETE CASCADE`. `rules.rs` was rewritten as subcommands
  `config`/`add`/`edit`/`remove`/`reorder`/`list`/`post`, all keyed on the
  invoking `guild_id`, using compile-time `query!`/`query_as!` on the concrete
  `PgPool` from `cx.app.db` (outside CC-1). `post` renders the rows into a
  `CreateEmbed` and edits the stored `message_id`, self-healing on
  `UnknownMessage`/`UnknownChannel` by sending a fresh message and persisting the
  new id. The hardcoded `CHANNEL_ID`/`MESSAGE_ID` consts, the
  `"College Kings Server Rules"` title/gist link, and the `messages/rules.md`
  read are all gone (that file was never tracked in-repo — runtime-only).
  **Residual:** `post` caps at 25 rules (one embed = 25 fields max) and rejects
  with a message rather than silently dropping — multi-embed paging is a
  follow-up if a guild ever needs >25 rules. No regression test: `bot` has no
  `[lib]` target, so integration tests against these bindings are structurally
  infeasible (same constraint as CC-6 / prior bot DS-1). The College Kings guild
  needs its rules rows seeded via the new command before its `/rules post` works
  (old on-disk content is not auto-migrated).
- **Where:** `bot/src/bindings/moderation/rules.rs` — `CHANNEL_ID`
  (`747430712617074718`) and `MESSAGE_ID` (`788539168980336701`) consts at
  `:17-18`; the `messages/rules.md` file read at `:37-41`; the hardcoded
  "College Kings Server Rules" title and Code-of-Conduct gist link at `:52-53`.
  (Flagged as a residual in DS-2, recorded here as its own finding.)
- **What:** The command has no per-guild state at all. It reads one fixed
  markdown file off the bot's working directory, splits it on `\r\n\r\n` into
  embed fields, and edits **one** hardcoded message in **one** hardcoded channel
  of **one** guild. Invoking `/rules` in any other guild either edits the
  College Kings message (if the bot is in that guild) or fails outright with a
  `10003 Unknown Channel` / `10008 Unknown Message` — after the ephemeral defer,
  so the moderator just sees an interaction error.
- **Why it matters:** every other module in the bot is guild-scoped via the DB;
  this one is a single-tenant leftover. It also makes the rules text
  deployment-coupled (editing rules means editing a file and redeploying,
  rather than a moderator command), and the `\r\n\r\n` split silently produces a
  single field if the file is ever saved with LF-only line endings.
- **Confidence:** confirmed by reading the file — no guild lookup, no DB access,
  no fallback path.
- **Suggested fix:** move rules to the database and make the command
  guild-generic:
  - New table (e.g. `guild_rules`): `guild_id` PK, `channel_id`, `message_id`
    (nullable — set on first post), plus embed presentation fields (`title`,
    `description`, `colour`), and a child `guild_rule` table
    (`guild_id`, `position`, `title`, `body`) so rules are ordered rows rather
    than a parsed blob.
  - `/rules` becomes a command group: a mod-only subcommand set to
    add/edit/remove/reorder rules and set the target channel + embed styling,
    and a `post`/`refresh` subcommand that renders the rows into a
    `CreateEmbed` and either edits the stored `message_id` or sends a new
    message and persists the returned ID (self-healing when the stored message
    is deleted — treat `10008` as "send fresh").
  - Access via compile-time `sqlx::query!`/`query_as!` in the binding, matching
    the rest of `bindings/moderation/` (concrete `PgPool` from `cx.app.db`, so
    still outside CC-1).
  - Delete `messages/rules.md` and the two magic-ID consts once the College
    Kings guild's rows are seeded.
