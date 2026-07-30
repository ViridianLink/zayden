# Audit: palworld

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Solid, recently-built crate: concrete `PgPool`, strong test coverage (12
integration files), and — importantly — its blocking save-file I/O is correctly
offloaded via `tokio::task::spawn_blocking` (`client.rs`, `commands/upload.rs`).
The type modelling (`model.rs` element enum with alias-tolerant `parse`) is a
good example of #4 done right. Minor: one inline test module and one blocking
`std::fs` call worth confirming is off the async path.

## Findings

### 1. Inline `#[cfg(test)]` module  ·  #6  ·  med
- **Where:** `src/commands/breed_plan.rs:147`.
- **What / Why / Fix:** See [CC-2](_cross-cutting.md#cc-2). Move to `tests/`
  (the crate already has 12 integration files — harness is established).

### 2. Confirm `cron.rs` `std::fs::remove_file` is not on the async reactor  ·  #3  ·  low
- **Where:** `src/cron.rs:16` (`std::fs::remove_file`), and the sync `std::fs`
  helpers in `src/save/mod.rs`.
- **What:** Most save I/O is wrapped in `spawn_blocking`; verify the cron
  cleanup `remove_file` (a single unlink) either runs inside a blocking context
  or is cheap enough to accept. `save/mod.rs` helpers appear to be called only
  from within `spawn_blocking` closures.
- **Why it matters:** A stray sync `remove_file` on the async reactor is a
  (small) stall; a single unlink is usually tolerable but worth a glance.
- **Suggested fix:** Confirm the call site; wrap if it runs on an async task.

### 3. Breed-plan / Paldex displays are better as dashboard read-views  ·  #8  ·  low
- **Where:** `src/commands/breed_plan.rs`, `src/commands/breed_for.rs`, Paldex
  display paths.
- **What:** Breeding-path and Paldex output is data-dense and better browsed on a
  web page than paged in an embed.
- **Why it matters:** UX gain; the breeding data is already computed from
  DB/model.
- **Suggested fix:** Add dashboard breed-plan/Paldex views; keep save-upload and
  live server ops in-bot. See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: `transport/` (fandom/pelican) + `save/` + `commands/` +
  `client.rs` cleanly separated; concrete `PgPool`.
- #1 DB access: compile-time macros; `.query(&[...])` are HTTP params, not SQL.
- #3 Async: **correct** — save decode/load offloaded via `spawn_blocking`.
- #4 Stringly typing: `model.rs` element enum has an alias-tolerant `parse`
  (handles source typos like `"electricty"`) — good.
- #6 Tests: 12 integration files (breeding, upload, save decode/world, guild).

## Addendum — save write path (2026-07-28)

The crate previously only ever read saves. It now has a write path, used by the
dashboard's admin save editor:

- **`save::compress`** — the inverse of `save::decompress`. Emits the 12-byte
  Palworld container header plus a zlib body. Only `PlZ` is written: the game's
  own saves are Oodle (`PlM`) and `oozextract` decompresses but cannot encode,
  so a re-exported save changes container format. `source_type_byte` reads the
  type byte off the original so single/double compression is preserved.
  Whether the game *accepts* `PlZ` in place of `PlM` cannot be proven from
  inside the repo — it is a manual check.
- **`save::gvas::reparse_properties_at`** — reads a `CharacterSaveParameterMap`
  `RawData` blob and reports where it stopped, keeping the bytes after the
  `"None"` terminator. Measured at 24 bytes on every one of the fixture's 1,822
  characters (padding plus a group-id GUID). `reparse_properties` discards that
  trailer and is now a thin wrapper over this; it remains correct for read-only
  callers, but writing from its output alone would truncate every character in
  the world. `write_properties` is the exact inverse and is asserted
  byte-identical across all 1,822 blobs.
- **`save::edit`** — `read_roster` produces an editable summary keyed by
  `InstanceId` (unique per character, unlike `PlayerUId`); `apply_edits` patches
  `Level`, `Exp`, the three `Talent_*` IVs and `PassiveSkillList` and re-emits a
  compressed save. Insertion is mandatory rather than optional: the game omits
  default-valued properties, so 176 of 1,822 characters carry no `Level` at all
  and 126 carry no `PassiveSkillList`. An unknown instance id is a hard error
  (`PalworldError::Edit`), the signal that the mirror moved under an in-flight
  edit.

Nothing in this path writes to disk. `apply_edits` returns bytes; the mirror is
opened read-only on every route.

Two behaviours are asserted by tests but not yet confirmed in-game: `PlZ`
acceptance, and that setting `Exp = 0` alongside a level change does not make
the game re-derive the level from accumulated experience and revert it.

## Addendum — level-up point grants (2026-07-29)

A level edit that only moved `Level` handed out a level without the points that
level would have earned. Both grants now travel with it.

**Status points — one per level, measured.** `save::edit::STATUS_POINTS_PER_LEVEL`
is not taken on trust. `tests/save_points.rs` asserts an identity across all 11
player characters in `progressed-world` and `storage-world`: the points
allocated to the five level-up stats in `GotStatusPointList` (`最大HP`, `最大SP`,
`攻撃力`, `所持重量`, `作業速度`) plus `UnusedStatusPoint` equal `level - 1`
exactly. It also held on all three characters in the live world. The other
entries in that list (`捕獲率`, `移動速度アップ`, …) and the whole of
`GotExStatusPointList` come from non-level sources and are excluded.

`steam-world1` is the one save that breaks the identity — 250 core points at
level 65 — and it is why `grant_status_points` moves the unspent pool by a delta
instead of recomputing the total from the level. A recompute would silently
rewrite a manipulated save's allocations, and the editor cannot assume the
identity holds on input it did not create.

**Technology points — six per level, from the wiki.** `TECH_POINTS_PER_LEVEL`
cannot be confirmed against a save: the earned total is only recoverable as
`TechnologyPoint + sum(cost of UnlockedRecipeTechnologyNames)`, and saves carry
no recipe costs. `bossTechnologyPoint` is never touched — Ancient Technology
Points come from first-time Alpha Pal (1) and Tower Boss (5) kills and from
Technical Manuals, so granting them for a level change would hand out progress
the grind never would.

**Two files, one archive.** Technology points live in `Players/<uid>.sav`, not
`Level.sav`, so `apply_edits` now returns `EditedSave { level, level_deltas }`
and the caller patches each moved player's file via
`save::edit_player::grant_tech_points`. The export route bundles them into a zip
(stored, not deflated — the payloads are already compressed containers) because
the two files are only valid together; a level-only edit still returns a bare
`Level.sav`. A player file that cannot be read is a hard error rather than a
partial export, since a save whose levels moved but whose points did not is
indistinguishable from a correct one at a glance.

**Level cap is 80, not 60.** Four characters across the live world and the
fixtures sit at exactly level 80 with byte-identical `Exp` (45859908), and the
live saves carry `ExpTableMigrationVersion = 1` — the game's own marker that it
migrated off the older, lower cap. `Level` is a `ByteProperty`, so the format
ceiling is 255.
