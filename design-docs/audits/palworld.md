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
- **Status:** `complete — 916dbcbf`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Confirmed, then fixed (2026-08-05).** The third untagged "confirm X" item,
  after [ai #1](ai.md) and [music #2](music.md) — and the third whose
  confirmation **failed**. Its lesson is that a "confirm" finding's cited
  surface is the *weakest* part of it: the code moved, and it moved the wrong
  way.
  - **The unlink is no longer an unlink.** The finding sized the risk as "a
    single `remove_file`, usually tolerable". `PalworldUploadSweepCron` now
    resolves each expired row's `file_path` to its **parent** and calls
    `std::fs::remove_dir_all` — `file_path` points at `Level.sav` *inside* the
    uploader's directory, so `parent()` is `Some` on every real row and the
    `remove_file` arm is unreachable in practice. What actually ran on the
    reactor was a recursive walk of a whole upload directory (up to 100 MB on
    Ultra), once per expired row, every minute.
  - **The blast radius is the whole tick, not one job.** `bot/src/cron.rs:72-76`
    collects every job due at the same instant and polls them together with
    `future::join_all` on **one** task. A sweep that blocks therefore stalls the
    co-scheduled jobs too — the entitlement expiry sweep shares its minute
    boundary (`0 0 * * * * *`), and palworld's own save refresh runs at
    `0 */2 * * * * *`.
- **Fix.** The removal loop moved out of the cron closure into
  `cron::remove_expired_uploads`, which hands the whole batch to one
  `tokio::task::spawn_blocking` — one hop off the reactor for the batch, not one
  per path — and logs a `JoinError` rather than dropping it. An empty sweep (the
  common case; the job fires every minute) returns before touching the pool.
  Per-path `warn!`-and-continue is preserved deliberately: the rows are already
  deleted by `delete_expired`'s `RETURNING`, so aborting the loop on one bad
  path would strand the remaining payloads on disk with no row left to find them
  by.
- **The `save/mod.rs` half confirms clean, as the finding predicted.**
  `write_atomic` (`:46`), `read_level` (`:64`) and `player_dir_uids` (`:170`)
  are reachable only through `load_world` / `load_world_with` /
  `load_player_names` / `write_raw_player` / `store`, and **every** call site is
  inside a `spawn_blocking` closure: `client.rs:287,338,373,412,537,550,575,599`,
  `commands/upload.rs:137`, `dashboard/src/server/palworld_save.rs:31`,
  `dashboard/src/web/routes_palworld_save.rs:48`. `dps::parse` /
  `parse_player_uid` at `client.rs:627-629` take `&[u8]` and touch no disk.
- **Regression test** `tests/upload_sweep.rs`, 3 tests, **1 fails-before / 3
  pass-after** in 0.00 s. The guard asserts the sweep's *first poll* is
  `Pending`: a future that unlinks inline is `Ready` from that poll, having
  already burned the reactor thread. Determinism comes from capping the runtime's
  blocking pool at one thread and occupying it before the sweep starts, so
  "the blocking work has not run yet" is a guarantee rather than a bet on the
  removal outlasting the first poll of its own join handle. The gate self-releases
  on a timeout so a regression **fails** instead of hanging on runtime shutdown.
  The other two cover the loop's resilience to a bad path and that the whole
  payload directory goes, not just the named `Level.sav`.
- **Gates:** `cargo +nightly clippy --workspace --all-targets -- -D warnings`
  clean (via bacon, `.bacon-locations` empty); `cargo test --workspace
  --no-fail-fast` 612 passed / 0 failed; `cargo +nightly fmt`. No SQL touched, so
  no `.sqlx` regen; no `Cargo.toml` change (the test needed only tokio features
  the crate already had), so no `cargo machete`; no dashboard code, so no feature
  checks. No new `#[allow]`/`#[expect]`.
- **Residual:** the sweep now bounds nothing else — a single upload directory
  large enough to stall a *blocking* thread still does so, which is the correct
  place for it. One genuinely new observation this pass surfaced is recorded
  separately as **finding #4** below.
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

### 4. `stat`/`read_dir` on the async path in the cache-key lookups  ·  #3  ·  low
- **Status:** `complete — 6d51f6fc`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Reconciled (2026-08-05):** the marker was left at `in-review` after the human
  committed the task as `6d51f6fc`. Verified against the tree, not the record:
  the commit carries `client.rs`, `save/dps.rs`, `save/mod.rs` and
  `tests/cache_key_offload.rs`, and `client.rs` now has no `std::fs::metadata`
  on `file_mtime`.
- **Confirmed and fixed (2026-08-05).** Both cited sites reproduced exactly as
  recorded; neither had moved since the finding was written the same day.
- **Fix — two different shapes, because the two sites are not the same size.**
  The finding suggested one remedy ("fold the stat into the `spawn_blocking`
  that already follows it") and named `level_mtime` as a second precedent. Which
  one applies turns on how much work the key derivation is:
  - `player_record` (`client.rs:330`) derives its key from **one** stat, and the
    `spawn_blocking` it precedes is skipped entirely when the save is missing or
    the cache already holds the record. Folding a single stat into that closure
    would mean spawning a blocking task to answer a question that decides whether
    to spawn one. `file_mtime` is now `async` over `tokio::fs::metadata` —
    `level_mtime` (`:641`), which had the identical job, already did this.
  - `storage_pals` (`client.rs:403-404`) is the opposite: a `read_dir` plus one
    stat *per* `*_dps.sav`, an unbounded batch, all of it before the first parse
    is spawned. That is worth its own hop, and it must be **one** hop for the
    directory rather than one per file — the same "batch, don't iterate the
    offload" lesson [#2](#2-confirm-cronrs-stdfsremove_file-is-not-on-the-async-reactor--3--low)
    recorded. It moved to `save::dps::list_files_with_mtime`, which returns
    `Vec<(PathBuf, u64)>` from a single `spawn_blocking`; the cache probe and the
    per-file parse spawns stay on the async side, unchanged.
- **`mtime_nanos` moved** from `client.rs` to `save/mod.rs` (now `pub`) so the
  listing helper and the two client-side callers share one definition rather than
  the key being derivable two ways. `list_files` itself is untouched and stays a
  pure listing.
- **Regression test** `tests/cache_key_offload.rs`, 3 tests, **2 fail-before /
  3 pass-after** in 0.02 s. The guard is `#2`'s: cap the runtime's blocking pool
  at one thread, occupy it, and assert the subject's **first poll** is `Pending`.
  A future that stats inline is `Ready` from that poll, having already spent the
  reactor thread.
  - `player_record` needed one extra idea to be observable. Its downstream
    `spawn_blocking` parse would return `Pending` on the first poll either way,
    masking the defect — so the test asks for a save that **does not exist**,
    where the lookup short-circuits to `Ok(None)` the moment the mtime comes back
    empty and nothing downstream can hide the stat. It is the same syscall on the
    cache-hit and parse paths; this is just the case that isolates it.
  - The helper deliberately does not re-poll a future that already returned
    `Ready`: doing so panics with "async fn resumed after completion", which
    masked the assertion message on the first fails-before run.
- **Gates:** `cargo +nightly clippy --workspace --all-targets -- -D warnings`
  clean (exit 0, and bacon's own pass exported an empty `.bacon-locations`);
  `cargo test --workspace --no-fail-fast` **615 passed / 0 failed** (612 before,
  plus these 3); `cargo +nightly fmt`. No SQL touched, so no `.sqlx` regen; no
  `Cargo.toml` change (`tokio`'s `fs` and `rt` features were already on), so no
  `cargo machete`; no dashboard code, so no feature checks. No new
  `#[allow]`/`#[expect]`.
- **Residual:** `local_mtime_secs` (`client.rs:636`) is the crate's third sync
  `std::fs::metadata`, and it was **checked and is clean** — both call sites
  (`:540`, `:578`) are already inside `spawn_blocking`. The Pelican refresh path
  needed no change.
- **Recorded 2026-08-05** by the [#2](#2-confirm-cronrs-stdfsremove_file-is-not-on-the-async-reactor--3--low)
  close-out. Not part of that finding — #2 named `cron.rs` and `save/mod.rs`, and
  both of those are now resolved — but found while proving its second half.
- **Where:** `src/client.rs:330` (`file_mtime` → `std::fs::metadata`),
  `src/client.rs:403-404` (`save::dps::list_files` → `std::fs::read_dir`, then
  `file_mtime` **per entry in a loop**).
- **What:** The save-parsing work in `player_record` and `storage_pals` is
  correctly offloaded (`spawn_blocking` at `:338` and `:412`), but the
  **cache-key** derivation that runs *before* it is not: the mtime stat and the
  `Players/` directory listing happen inline on the reactor. `storage_pals`
  does one stat per `*_dps.sav` in the loop at `:403`.
- **Why it matters:** Much smaller than #2 — these are bounded metadata
  syscalls, not a recursive delete — but they sit on **user-facing command
  paths** (`/pal` lookups), not an unattended cron, and they run on *every*
  call including cache hits, since the mtime is what forms the key. On a network
  mount (the shared world is a server mirror) a stat is not reliably fast.
- **Suggested fix:** Fold the stat/listing into the `spawn_blocking` that
  already follows it, so one hop covers key derivation and load together.
  Note `level_mtime` (`:641`) already does this correctly with
  `tokio::fs::metadata` — a second precedent to follow if the key must be
  derived before the decision to load.

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
