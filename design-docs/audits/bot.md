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
- **Where:** `src/bindings/{gambling,levels,gold_star,family,…}/…` —
  `impl XxxManager<Postgres> for XxxTable` bodies.
- **What:** Because the manager traits are DB-generic, each module's SQL is
  implemented here in `bot/` rather than in the module crate. This is the
  scattering described in [CC-1](_cross-cutting.md#cc-1) — resolved when those
  modules go concrete and their SQL moves home.
- **Note:** `gold_star.rs` additionally uses runtime SQL — see
  [CC-5](_cross-cutting.md#cc-5) / [gold-star.md](gold-star.md).

### 4. Handful of `unwrap()`/`expect()` + a correctness TODO  ·  #3 / #2  ·  low
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
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
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
- **Status:** `in-review` (revived)            <!-- open | in-progress | in-review | complete | wontfix -->
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
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
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
