# `progressed-world` fixture

A real, fully-progressed Palworld shared world (Feybreak-era, captured
2026-07-28), committed so the save-reading tests have a fixed ground truth
instead of depending on a working directory that may be cleaned up, or rewritten
by the server mid-read.

```
Level.sav                  2.6 MB  Oodle-compressed (PlM), ~33 MB of GVAS
LevelMeta.sav                      not read by this crate; kept for completeness
Players/<uid>.sav          8       per-player saves - all progression lives here
Players/<uid>_dps.sav      4       Dimensional Pal Storage; see `save::dps`
```

This save predates the storage rework, so its `_dps.sav` files hold only a
handful of Pals and its `Level.sav` has no placeholder-owned stubs. Use
`storage-world` for anything that depends on the current format.

Eight players across four guilds, two of them multi-member with hundreds of
pooled base Pals - which is what makes the guild-isolation regression in
`tests/progress.rs` meaningful. Character completion ranges from a fresh
character to one with a full 303-entry Paldeck.

Player UID filenames are in the game's spelling (first three GUID groups
reversed); `save::uid_to_filename` converts to the byte order this crate stores.

Resolve the path through `tests/common::progressed_world()` rather than joining
it by hand.
