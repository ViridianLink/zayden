# Progress catalogues

Static reference data for `/palworld progress`. These are the *denominators* — the
full set of fast-travel points, bosses, effigies/relics, technologies, missions,
map areas and lab research the game ships — against which a player's save flags
are counted. They are embedded at compile time by `src/progress/catalogue.rs`
(`include_str!`), so the command needs no network and no database.

## Provenance

Derived and trimmed from two MIT-licensed projects:

- [oMaN-Rod/palworld-save-pal](https://github.com/oMaN-Rod/palworld-save-pal) —
  `data/json/` (ids, classes, world coordinates) and `data/json/l10n/en/`
  (display names).
- [deafdudecomputers/PalworldSaveTools](https://github.com/deafdudecomputers/PalworldSaveTools) —
  `resources/game_data/` (`boss_mapping.json` bounty tokens, `world_map_areas.json`,
  the 11 newer fast-travel points psp does not yet carry).

Only the fields the progress computation needs are kept, and coordinates are
rounded to one decimal.

## Files

| File | Entries | Keyed by | Matches save field |
|---|---|---|---|
| `fast_travel_points.json` | 174 | instance GUID | `RecordData.FastTravelPointUnlockFlag` |
| `bosses.json` | 125 | `spawner` | `RecordData.NormalBossDefeatFlag` |
| `relics.json` | 406 | instance GUID | `RecordData.RelicObtainForInstanceFlag(ByType)` |
| `relic_types.json` | 13 | relic type key | `RecordData.RelicPossessNumMap` |
| `technologies.json` | 591 | technology id | `SaveData.UnlockedRecipeTechnologyNames` |
| `missions.json` | 120 | quest id | `SaveData.CompletedQuestArray_FullRelease` |
| `areas.json` | 125 | area id | `RecordData.FindAreaFlagMap` |
| `towers.json` | 13 | `BOSS_BATTLE_NAME_*` | `RecordData.TowerBossDefeatFlag` |
| `pals.json` | 303 | character id | `RecordData.PaldeckUnlockFlag`, `PalCaptureCount` |

`bosses.json` carries `alpha` (89 `BOSS_`-prefixed field alphas) and `bounty`
(89 spawners that drop a `BossDefeatReward_*` bounty token). The 154
`capture_power` entries in `relics.json` **are** the Lifmunk Effigies — the save
format calls effigies "relics" for historical reasons, and the flat
`RelicObtainForInstanceFlag` map tracks exactly those.

## The `map` field

The game ships **two maps**, and the save pools them into one flat set of flags.
`fast_travel_points.json`, `bosses.json`, `relics.json` and `towers.json` each
carry a `map` of `palpagos` or `tree` so progress can be reported per map — a
World Tree statue is not a Palpagos statue the player has yet to find, and its
coordinates land in open water on the main map.

The split comes from the game's own `DT_WorldMapUIData` rectangles (see
`MAP_AREAS` in `generate.py`), with `Tree` taking priority in the sliver where
they overlap, exactly as the game does. Player saves confirm the model: a save
that has reached the region gains `Tree` alongside `MainMap` in
`RecordData.UnlockedWorldMapFlags`.

| Catalogue | Palpagos | World Tree |
|---|---|---|
| `fast_travel_points.json` | 157 | 17 |
| `bosses.json` | 118 | 7 |
| `relics.json` | 359 | 47 |
| …of which effigies | 139 | 15 |
| `towers.json` | 9 | 4 |

Towers carry no coordinates, so their `map` is assigned by hand in `generate.py`.
The one effigy neither upstream dump gives a position for has no `map` key at
all; `Region`'s serde default counts it with Palpagos, which keeps the effigy
totals summing to 154.

`areas.json` is **not** split: map discovery is a Palpagos-only mechanic. The
list holds `FootOfWorldTree` (the approach, on the main map) and nothing on the
World Tree itself, which the eight real saves confirm.

`pals.json` is the Paldeck denominator: 303 real, deck-indexed Pals across 204
dex numbers (variants share a number). It deliberately excludes the humans and
raid monsters that also appear as `PalCaptureCount` keys.

Lab research is deliberately **not** carried: `LabResearchInfo` appears nowhere
in a fully-progressed 2026-07 `Level.sav`, so a milestone built on it would
always read 0. Re-check before adding it back.

`tests/progress_catalogue.rs` asserts every count above, so a bad re-sync fails
loudly rather than silently skewing percentages.

## Validated against real saves

Every catalogue was checked key-for-key against eight progressed player saves
(world `056C426C…`, July 2026). Coverage of the ids those saves actually carry is
100% for fast travel, bosses, areas, technologies, quests and effigies. Ids the
upstream dumps were missing are folded back in explicitly — see `EXTRA_*` in
`generate.py`. Paldeck and boss ids must be matched **case-insensitively**: the
save writes `SheepBall`/`WereWolf_Ice` where psp records `Sheepball`/`Werewolf_Ice`.

## Re-syncing after a game update

Run `generate.py` from a directory holding fresh `psp/` and `pwst/` clones. Update
the counts in `tests/progress_catalogue.rs` in the same change, and re-check the
`EXTRA_*` lists — entries that upstream has since added should be dropped from
them.

If an update adds a third map, `MAP_AREAS` and `TOWERS` need the new rectangle
and `Region` needs the new variant; `generate.py` prints a `by map` line per
catalogue, and anything landing in the `None` bucket is a position that fits no
rectangle and wants investigating.
