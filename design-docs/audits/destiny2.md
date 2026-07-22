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
- **Where:** `src/endgame_analysis/tierlist.rs`, `src/loadouts/*` render paths.
- **What:** Loadout *editing* already moved to the website (M3 3c). The read side
  — tier lists and browsable loadouts — is data-dense catalog content that a web
  page presents better than embeds.
- **Why it matters:** Completes the destiny2→web direction already in motion; the
  catalog is DB-backed already, so a read view is cheap.
- **Suggested fix:** Add dashboard browse/tier-list views; keep autocomplete +
  `builds refresh` in-bot. See [CC-8](_cross-cutting.md#cc-8).

## Deep-sweep findings

_Deep sweep, 2026-07-17 (fourth pass — drilled the previously "essentially clean"
loadout render path). Two confirmed/plausible latent defects underneath the
first-pass structural findings._

### DS-1. `/destiny2 builds` holds the `RwLock<BotState>` **write** guard across emoji upload + `sleep` (≤50 s) and never defers  ·  #3 (async) + Discord-ack  ·  high
- **Status:** `complete — 095edd7b`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `src/loadouts/record.rs:93-99` (write guard `let mut data =
  data_lock.write().await;` acquired, held to end of `into_component`) plus the
  ~14 `resolve_emoji(...).await` calls at `record.rs:104-238`; the retry loop
  `src/loadouts/mod.rs:225-231` (`emoji_cache.upload(...).await` then
  `tokio::time::sleep(Duration::from_secs(5))`, `MAX_ATTEMPTS = 10`);
  `Loadout::run` at `src/loadouts/mod.rs:103-142` issues `create_response`
  (Message, `IS_COMPONENTS_V2`) with **no** prior `defer` — contrast
  `Perk`/`Weapon`/`TierList`/`DimWishlist`, which all `defer` first
  (`slash_commands/perk.rs:32`, `endgame_analysis/{weapon,tierlist,dimwishlist}.rs`).
- **What:** two coupled defects.
  1. **Lock held across `.await` (network + sleep).** `into_component` takes the
     global `RwLock<BotState>` write guard (`emojis_mut()` needs `&mut` →
     `Arc::make_mut`, `bot/src/state.rs:162`) and keeps it while awaiting
     `resolve_emoji`, which on any cache miss does a Discord CDN download + a
     `create_application_emoji` HTTP call + `sleep(5 s)`, retried up to 10×. Every
     other `BotState` accessor blocks for the whole window:
     `bot/src/handler/voice_state_update.rs:17` takes `.read().await` on **every**
     voice join/leave (occupancy + music auto-disconnect), all music
     command/component handlers, gambling emoji reads (`data.emojis()`), palworld,
     and `guild_create` (`state.rs:150`). One cold-emoji loadout render stalls the
     bot's shared state for 5 s per missing emoji, up to ~50 s if an emoji name is
     unresolvable (absent on the parent app → `upload` just warns and returns, so
     all 10 attempts merely sleep). The `await_holding_lock` clippy lint does **not**
     fire because this is a `tokio::sync::RwLock`, not a std/parking_lot lock — which
     is why the first-pass "Clean · #3 … no locks across `.await`" line missed it.
  2. **No defer → ack timeout.** `run` performs the full `into_component` render
     before its first `create_response`. With any cache miss the 5 s sleep(s)
     exceed Discord's 3 s ack deadline; the interaction token expires and
     `create_response` fails with `10062 Unknown Interaction`. The user sees "This
     interaction failed" and the emoji-unavailable error path
     (`resolve_emoji`'s final `Err`) can't be delivered either.
- **Failure scenario:** A loadout row references an emoji not yet uploaded to
  Zayden's own application (a freshly added build, or a renamed emoji). A user runs
  `/destiny2 builds warlock <build>`. `EmojiCache::emoji(key)` misses →
  `resolve_emoji` uploads + `sleep(5 s)` under the write guard. (a) The 3 s ack
  window is blown, so the final `create_response` returns 10062 and the command
  visibly fails. (b) Meanwhile another user joins a voice channel:
  `voice_state_update` blocks on `.read().await` until the render releases the
  guard, and any music/gambling component pressed in that window hangs too. If the
  emoji name is stale/typo'd (never on the parent), the loop runs 10 × 5 s = 50 s,
  holding the write lock the entire time and failing every single invocation
  (never cached, so it recurs).
- **Confidence:** confirmed (both the lock-across-await and the missing-defer read
  directly from source; the deferring siblings confirm the intended pattern).
  Impact magnitude scales with how often loadout emojis miss the warm cache.
- **Suggested fix:** `defer()` at the top of `Loadout::run` like the sibling
  subcommands and switch to `edit_response`. Drop the `RwLock<BotState>` write
  guard before awaiting: resolve/upload any missing emojis first (or clone the
  `Arc<EmojiCache>` snapshot) and build the components without holding the guard
  across `resolve_emoji`. Bound or delete the 10 × 5 s retry — it cannot succeed
  for an emoji absent on the parent app. Correct the "Clean · #3" line below.

### DS-2. `compendium::update` panics (`Vec::swap_remove(2)`) on any short "gear perks" row  ·  #2 / #3  ·  low-med
- **Status:** `complete — b84e49da`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-22):** The inline row-parse closure in `compendium::update`
  (`swap_remove(2)`/`swap_remove(0)` on `row.values`) was extracted into a pure
  `pub fn perk_entry(Vec<Option<String>>) -> Option<(String, PerkInfo)>` that
  guards `values.len() < 3` and returns `None` for short/blank rows instead of
  indexing out of bounds. `update` now maps each cell to its `formatted_value`
  and routes through `perk_entry`. Regression test
  `tests/compendium_parse.rs` (fails-before: `swap_remove index (is 2) should be
  < len (is 0)`; passes-after: short rows skipped, full rows parse).
- **Where:** `src/compendium.rs:38-41` — `values.swap_remove(2).formatted_value`
  then `values.swap_remove(0).formatted_value`.
- **What:** `Vec::swap_remove` panics on an out-of-bounds index. The Google Sheets
  API omits trailing empty cells, so any perk row (after `skip(5)`) with fewer
  than three populated cells — a blank section divider, or a name-only row with no
  description column — makes `swap_remove(2)` panic. Workspace lints deny the
  `panic!` macro but not a library method that panics, so this compiles.
- **Failure scenario:** The "gear perks" tab gains a blank/short row. On the next
  `/destiny2 perk` (or its autocomplete) while `destiny2_compendium_perks` is
  empty, `run`/`autocomplete` call `compendium::update`; the row iterator hits the
  short row → panic unwinds the interaction task, the `replace` transaction never
  commits, the table stays empty, and every subsequent perk invocation re-runs
  `update` and re-panics — persistent breakage until the sheet is fixed.
- **Confidence:** plausible (panic path confirmed from source; reachability
  depends on the external sheet containing a <3-cell data row past row 5).
- **Suggested fix:** read cells with `values.get(0)` / `values.get(2)` (or guard
  `values.len() >= 3`) and skip the row via the existing `filter_map` instead of
  `swap_remove`.

## Clean
- #1 Architecture: `db/{mod,endgame,compendium,loadouts}.rs` cleanly separated;
  concrete `PgPool`; no DB-generic trait (not subject to CC-1).
- #1 DB access: compile-time `query!`/`query_as!` only; transactional `replace`.
- #3 Async: no blocking I/O (moved off `fs::` in 3a). No locks held across
  `.await` — **DS-1 fixed (in-review)**: `loadouts/record.rs` now renders against
  an owned `EmojiCache` snapshot (cloned under a brief read guard, merged back
  under a brief write guard), so the `tokio::sync::RwLock<BotState>` guard is no
  longer held across `resolve_emoji`'s upload/network. (The `await_holding_lock`
  lint does not catch tokio locks, so this class must be guarded by review.)
- #4 Stringly typing: `Affinity`/`TierLabel`/`Frame`/`Class`/`Element`/`Mode`/
  `StatKind`/`Archetype` are all typed enums with round-trip tests.
- #5 Data placement: catalog + loadouts + endgame/compendium all DB-backed
  (only raid-guides render outstanding — finding #1).
- #6 Tests: three integration files in `tests/`, real round-trip coverage.

### DS-3. Endgame sheet parse failures silently drop weapons, and `replace` (`TRUNCATE`) makes it destructive → tierlist/perk data erodes as the source sheet drifts  ·  Pass 1 (silent failure) + SQL integrity  ·  med
- **Status:** `complete — 6514f6fc`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** parser `endgame_analysis/sheet/frame.rs:42-84` (`Frame::from_str`),
  `endgame_analysis/sheet/weapon.rs:238` (`perk 1 cell value`) and
  `weapon.rs:299-306` (`frame parse`); silent-skip at
  `endgame_analysis/sheet/mod.rs:126-149` (`.inspect_err(…).ok()`); destructive
  persistence at `db/endgame.rs:91-129` (`TRUNCATE destiny2_endgame_weapons
  RESTART IDENTITY` then re-insert only the survivors).
- **What:** `Frame::from_str` only matches **RPM-qualified** frame strings
  (`"Dynamic (140RPM)"`, `"Balanced (260RPM)"`, …). The upstream sheet now emits
  **bare** `"Dynamic"` / `"Balanced"` (the RPM appears to have moved to a separate
  column), so `from_str` returns `Err(())` → `WeaponBuilder::build` returns
  `missing_data("frame parse")` → the weapon is dropped by the `.ok()` at
  `mod.rs:145-148`. Same for `missing_data("perk 1 cell value")`. Because the cron
  refresh calls `endgame::replace`, which **TRUNCATEs** the table and re-inserts
  only the weapons that parsed, each dropped weapon is *removed from the live DB* on
  refresh — `/tierlist` and `/perk` progressively lose weapons as the sheet format
  drifts, with only an `error!` line to show for it.
- **Failure scenario (production-confirmed, 2026-07-17):** logs show
  `Failed to parse: 'Dynamic'`, `Failed to parse: 'Balanced'`,
  `Skipping weapon build in 'SMGs': missing data: frame parse`,
  `Skipping weapon build in 'Scouts': missing data: frame parse`,
  `Skipping weapon in 'Swords': missing data: perk 1 cell value`. Each of those
  weapons is absent from the table after the refresh's TRUNCATE+reinsert. Worst
  case: an upstream reformat that breaks many rows (or a header change that fails a
  whole sheet at `mod.rs:121-123`) leaves `weapons` nearly empty, and the
  unconditional TRUNCATE wipes the catalog to that tiny set → the endgame feature
  goes largely blank until the parser is patched. There is **no** guard that the
  freshly-parsed set is plausibly complete before replacing.
- **Why it matters:** this is a live, ongoing data-completeness regression on a
  user-facing feature (the tierlist/perk lookups silently under-report), and the
  TRUNCATE turns every parser/upstream drift into *data deletion* rather than a
  no-op. Note the Clean §#4 claim ("`Frame` … typed enums with round-trip tests")
  is true but insufficient: the round-trip test proves `enum→str→enum`, not that
  the enum covers the sheet's *current* vocabulary.
- **Confidence:** confirmed (parser mismatch read directly against the production
  log strings; TRUNCATE-then-insert semantics read in `db/endgame.rs`).
- **Suggested fix:** (1) reconcile `Frame::from_str` with the current sheet — accept
  bare `"Dynamic"`/`"Balanced"` (combining a separate RPM column) or add the missing
  variants, and add a parser test seeded from a *current* sheet snapshot, not only
  the enum's own strings; (2) make the refresh non-destructive on partial parse —
  upsert-by-name instead of `TRUNCATE`, or guard the `TRUNCATE` behind a sanity
  check (parsed count > 0 and not a large drop vs. the existing row count) so a
  transient upstream change can't erase the catalog. Same "sheet drift breaks the
  parser" theme as [DS-2](#ds-2-compendiumupdate-panics-vecswap_remove2-on-any-short-gear-perks-row) (different table).
