# Audit: destiny2

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Healthy and the most recently modernised module: concrete `PgPool`, compile-time
`query!`/`query_as!` throughout the `db/` layer, `sqlx::Type` domain enums, and
three integration test files (`endgame_types`, `loadout_domain`,
`loadout_refresh`). The residual debt is concentrated in the **raid-guides
subtree**, which the implementation spec knowingly deferred: it is still a
fully-`const`, panic-on-invariant builder and is the only part of the crate not
yet on the DB. Does not exhibit CC-1 (already concrete).

## Findings

### 1. Raid-guide render pipeline still fully `const`  ·  #5  ·  med
- **Status:** `open`
- **Where:** `src/raid_guides/mod.rs`, `raid_guides/weapons.rs`,
  `raid_guides/{last_wish,desert_perpetual}.rs`.
- **What:** #4's data-to-DB move seeded `destiny2_raid_weapons`, but the render
  path still reads `const` emoji/CDN tables instead of the DB rows. Explicitly
  flagged as deferred in the TODO (M3 3b).
- **Why it matters:** Splits raid-weapon data across a DB table and a parallel
  `const` set — the exact inconsistency #4 set out to remove. An admin editing
  the table (the stated website-CRUD end state) won't see raid-guide changes.
- **Suggested fix:** Port `raid_guides` to async DB reads against
  `destiny2_raid_weapons` (mirror `db/loadouts.rs`), then delete the const
  tables. Small (2 rows) but closes the #4 loop.

### 2. `const fn` builders panic on invariant, behind `#[expect]`  ·  #2 / #7  ·  low
- **Status:** `open`
- **Where:** `src/raid_guides/mod.rs:61-87,197-201` — `add_weapon` etc. use
  `#[expect(clippy::indexing_slicing)]` + `#[expect(clippy::panic)]` with
  `panic!("Encounter list is full")`.
- **What:** Compile-time-invariant panics silenced by paired `#[expect]`s (part
  of CC-3).
- **Why it matters:** The `reason`s are legitimate (build-time invariants), but
  the whole builder disappears once finding #1 moves this data to the DB.
- **Suggested fix:** Resolve as a side effect of #1; don't invest in the const
  builder.

### 3. Two archetype representations (intentional, document it)  ·  #4  ·  low
- **Status:** `open`
- **Where:** `src/loadouts/domain.rs:129` (`Archetype` `sqlx::Type` enum) vs.
  `src/endgame_analysis/sheet/weapon.rs` (archetype kept as free-text `String`).
- **What:** The TODO (M3 3b) records the deliberate decision not to unify these:
  the endgame sheet's archetype is genuinely unbounded free text, the loadout
  archetype is a closed `destiny2_archetype` enum.
- **Why it matters:** Not a bug, but a future auditor will re-flag it. Worth a
  one-line comment at the `sheet/weapon.rs` field pointing at the enum and saying
  why it stays a `String`.
- **Suggested fix:** Add the explanatory comment; no code change.

### 4. Tier-list / loadout browsing are better as dashboard read-views  ·  #8  ·  low
- **Status:** `open`
- **Where:** `src/endgame_analysis/tierlist.rs`, `src/loadouts/*` render paths.
- **What:** Loadout *editing* already moved to the website (M3 3c). The read side
  — tier lists and browsable loadouts — is data-dense catalog content that a web
  page presents better than embeds.
- **Why it matters:** Completes the destiny2→web direction already in motion; the
  catalog is DB-backed already, so a read view is cheap.
- **Suggested fix:** Add dashboard browse/tier-list views; keep autocomplete +
  `builds refresh` in-bot. See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: `db/{mod,endgame,compendium,loadouts}.rs` cleanly separated;
  concrete `PgPool`; no DB-generic trait (not subject to CC-1).
- #1 DB access: compile-time `query!`/`query_as!` only; transactional `replace`.
- #3 Async: no blocking I/O (moved off `fs::` in 3a). No locks held across
  `.await`: `loadouts/record.rs` renders against an owned `EmojiCache` snapshot
  (cloned under a brief read guard, merged back under a brief write guard), so
  the `tokio::sync::RwLock<BotState>` guard is not held across `resolve_emoji`'s
  upload/network. (The `await_holding_lock` lint does not catch tokio locks, so
  this class must be guarded by review.)
- #4 Stringly typing: `Affinity`/`TierLabel`/`Frame`/`Class`/`Element`/`Mode`/
  `StatKind`/`Archetype` are all typed enums with round-trip tests.
- #5 Data placement: catalog + loadouts + endgame/compendium all DB-backed
  (only raid-guides render outstanding — finding #1).
- #6 Tests: three integration files in `tests/`, real round-trip coverage.
