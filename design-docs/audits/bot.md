# Audit: bot

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

The binary crate (~10k LOC, 99 files) that wires every module into Serenity:
`bindings/*` (the `ModuleComponent`/`ModuleModal` impls + the concrete `…Table`
manager impls), the `moderation` feature (which lives here, not in a crate), the
event `handler`, and the `registry`/`cron` scaffolding. Health is decent; the
residual issues are the remaining `#[expect]` escape-hatch and the structural
inability to host integration tests (no lib target).

## Findings

### 1. `#[expect]` cluster across bindings  ·  #7  ·  med
- **Status:** `unclear`
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

## Clean
- #1 Architecture: `bindings/` per-module, `handler/`, `registry/`, `cron.rs`
  cleanly separated; `ModuleComponent`/`ModuleModal` routing consistent.
- #1 DB access: bindings use compile-time macros.
- #2 Moderation (`InfractionKind` `sqlx::Type`, `NO_REASON`, `LogFilter`) — the
  magic-string cleanup is done, and the tree is **live** (registered via
  `moderation::register`), no longer dead code.
- #4 Stringly typing: mostly typed; residual `custom_id.as_str()` routing in
  `bindings/gambling/{prestige,blackjack}.rs` (CC-7).

## Deep-sweep findings

_Deep sweep (sixth pass): 2026-07-17. Defects in the `bot` wiring layer that no
per-crate audit could see, because they live in the glue between a module crate
and its binding — exactly the blind spot CC-1/CC-9 describe._

### DS-1. Level-up coin reward is a second transaction after XP is already committed → reward silently lost  ·  Pass 1 (silent failure) / SQL atomicity  ·  med
- **Status:** `unclear`            <!-- open | in-progress | in-review | complete | wontfix -->
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
