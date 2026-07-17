# Audit: reaction-roles

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Small (~500 LOC), clean `command/` + `reaction/` + manager split. Carries the
DB-generic `async_trait` pattern (CC-1) and no `tests/`. Otherwise unremarkable.

## Findings

### 1. DB-generic `async_trait` manager  ·  #1  ·  med
- **Where:** `src/reaction_roles_manager.rs`, `src/command/*`, `src/reaction/mod.rs`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1). Small surface — a
  good early migration.

### 2. No integration tests  ·  #6  ·  low
- **Where:** no `tests/` directory.
- **What / Why / Fix:** Add coverage for the emoji→role mapping resolution. See
  [CC-6](_cross-cutting.md#cc-6).

### 3. `add`/`remove` mapping CRUD belongs on the dashboard  ·  #8  ·  med
- **Where:** `src/command/{add,remove}.rs`.
- **What:** Managing message→emoji→role mappings is admin CRUD of reference data
  — a browsable web list is far better than paired slash commands.
- **Why it matters:** CRUD of config data is the dashboard's sweet spot (cf. the
  destiny2 loadout move); a table view makes the whole mapping visible at once.
- **Suggested fix:** Build a reaction-roles page (list/add/remove maps) against the
  concrete manager; keep the reaction **event handler** in-bot (it needs the live
  reaction). See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: clean command/reaction/manager separation.
- #1 DB access: concrete impl uses compile-time macros (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
- #4 Stringly typing: none of note.

## Deep-sweep findings

_Deep sweep (sixth pass): 2026-07-17 · lenses: silent-failure, state/orphan._

### DS-1. Reaction handler never skips the bot's own reaction → bot is granted every reaction-role  ·  Pass 7 (state) / #8  ·  low
- **Where:** `src/reaction/mod.rs:9-36` (`reaction_add`); seeded by
  `src/command/add.rs:66` (`message.react(http, reaction)`).
- **What:** After `/reaction-roles add` writes the mapping, the bot reacts to the
  panel message to seed the emoji. That `ReactionAdd` fires with the **bot** as the
  reactor; `reaction_add` looks the mapping up (now present), takes
  `reaction.member` (the bot's member, populated on guild reactions) and
  `add_role`s the reaction-role **to the bot**. There is no
  `if reaction.member…user.bot { return }` guard, unlike the usual reaction-role
  pattern.
- **Failure scenario:** admin runs `/reaction-roles add role:@Verified emoji:✅`.
  The bot posts the panel, reacts ✅, and immediately grants itself `@Verified`.
  Repeat for each mapping → the bot accumulates every reaction-role. Harmless in
  most setups but pollutes the bot's roles and can hand it roles above its intended
  scope.
- **Confidence:** confirmed (`add` seeds the reaction; handler has no bot filter).
- **Suggested fix:** early-return in `reaction_add`/`reaction_remove` when the
  reactor is a bot (or specifically the current user).
