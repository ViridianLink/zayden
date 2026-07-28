# `storage-world` fixture

A real Palworld shared world captured 2026-07-28 from the **current** game build -
the one that introduced Dimensional Pal Storage. `progressed-world` is a
Feybreak-era save and no longer represents what the server writes; this fixture
is what pins the current format.

```
Level.sav                  3.2 MB  Oodle-compressed (PlM), ~56 MB of GVAS
LevelMeta.sav                      not read by this crate; kept for completeness
Players/<uid>.sav          3       per-player saves - all progression lives here
Players/<uid>_dps.sav      2       Dimensional Pal Storage: the Pals themselves
```

Three players (KingJosh, Kitty, Oscar Six). What this world exercises that
`progressed-world` cannot:

- **Dimensional Pal Storage.** 214 Pals live in the `_dps.sav` files rather than
  in `Level.sav`, which keeps only stubs owned by the placeholder UID
  `save::GLOBAL_STORAGE_UID`. A reader that ignores these files under-counts
  Kitty by 197 Pals and invents a fourth "player" named after the placeholder.
- **`FoundTreasureMapPointMap`** (Oscar Six) - the per-instance treasure record
  that replaced the `FoundTreasureCount` scalar, and the struct-keyed map whose
  missing GVAS hint made the whole save undecodable.
- **`ArenaSoloClearCount`** (KingJosh, Oscar Six) - the five solo arena ranks.
- **`MutationCount`** (all three).

Player UID filenames are in the game's spelling (first three GUID groups
reversed); `save::uid_to_filename` converts to the byte order this crate stores.

Resolve the path through `tests/common::storage_world()` rather than joining it by
hand.
