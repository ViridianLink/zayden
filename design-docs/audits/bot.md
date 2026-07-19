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
  `src/bindings/temp_voice/mod.rs:132`,
  `src/bindings/moderation/infraction.rs:210`
  (`#[allow(clippy::too_many_arguments)]`).
- **What / Why / Fix:** See [CC-3](_cross-cutting.md#cc-3). The
  `too_many_arguments` on the infraction writer is the clearest refactor target
  (bundle the infraction fields into a struct).

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
- #2 Dead code (moderation): M2 landed `InfractionKind` (`sqlx::Type`),
  `NO_REASON` const, and the `LogFilter` enum — the magic-string cleanup is done.
- #4 Stringly typing: mostly typed; residual `custom_id.as_str()` routing in
  `bindings/gambling/{prestige,blackjack}.rs` (CC-7).

## Deep-sweep findings

_Deep sweep (sixth pass): 2026-07-17. Two new defects in the `bot` wiring layer
that no per-crate audit could see, because both live in the glue between a module
crate and its binding — exactly the blind spot CC-1/CC-9 describe._

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
