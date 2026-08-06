# Audit: temp-voice

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Recently extended with the M4 button control-panel; structure is clean
(`actions/` shared-mutation layer, `components/` one-file-per-button-group,
`commands/`, `events/`). Two residual issues: it still carries the DB-generic
manager pattern (CC-1) and has an inline `#[cfg(test)]` module (CC-2). Test
coverage is thin for the size of the crate — one `components.rs` structural test,
no coverage of the `actions` layer where the M4 permission re-checks live.

## Findings

### 1. DB-generic `async_trait` managers  ·  #1  ·  high
- **Status:** `complete — 611d350b`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-24):** CC-1 concrete-`PgPool` migration (sixth module after the
  `gold-star`/`levels`/`reaction-roles`/`suggestions`/`family` pilots). Dropped both
  generic manager traits — `TempVoiceGuildManager<Db: Database>` and
  `VoiceChannelManager<Db: Database>` — and the `#[async_trait]` on each. The SQL now
  lives in the crate as concrete `PgPool` associated functions:
  `TempVoiceRow::{save,get,get_category,get_creator_channel}` (guild settings) and
  `VoiceChannelRow::{get,count_persistent_channels,claim,save,delete}`. The
  `VoiceChannelManager` concrete impl (`VoiceChannelTable`) and the binding-only
  `TempVoiceMode` newtype (custom `temp_voice_mode` pgtype wrapper) moved out of
  `bot/src/bindings/temp_voice/mod.rs` into `voice_channel_manager.rs` alongside the
  query that needs it; `GuildTable`'s impl already lived in-crate. Every
  command/component/action/event `fn` lost its `<Db, Manager>` /
  `<Db, GuildManager, ChannelManager>` generics and now takes `&PgPool`; the
  `save`/`delete` row-wrappers became inherent `VoiceChannelRow` methods
  (`row.save(pool)` / `row.delete(pool)`). `bot/src/bindings/temp_voice/{commands,
  components,events,mod}.rs` drop their `::<Postgres, GuildTable, VoiceChannelTable>`
  turbofish, and `mod.rs` is reduced to `register` + the component/modal `use` list
  (the trait impl, `VoiceChannelTable`, and `TempVoiceMode` deleted). Removed the
  now-unused `async-trait` dependency (`cargo machete` clean). **Behaviour-preserving:**
  every `query!`/`query_as!`/`query_scalar!` string was moved byte-identically —
  verified each against its `.sqlx` cache entry by SHA-256 — so the offline cache is
  reused unchanged (`git status .sqlx` clean, no regeneration needed). The existing
  DB-free `tests/{components,ownership}.rs` and the inline `voice_channel_manager.rs`
  unit test are untouched and pass. Only `gambling`, `lfg`, and the `zayden-core`
  traits now remain on CC-1.
- **Where:** `src/voice_channel_manager.rs`, `src/guild_manager.rs`, and the
  `actions/*` + `components/*` that thread `<Db>` / `Pool<Db>`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1). Along with `lfg`, the
  closest structural sibling to the already-migrated `ticket`, so a good
  reference migration.

### 2. Inline `#[cfg(test)]` modules  ·  #6  ·  med
- **Where:** `src/voice_channel_manager.rs:168`, `src/commands/mod.rs:610`.
- **What / Why / Fix:** See [CC-2](_cross-cutting.md#cc-2). Move to
  `tests/`.

### 3. Region list hardcoded, flagged for API sync  ·  #4 / #5  ·  low
- **Where:** `src/components/mod.rs:43` — `// TODO: Can regions be pulled from
  Discord API to avoid future drift`.
- **What:** The voice-region option set is a hardcoded constant list that can
  drift from Discord's actual regions.
- **Why it matters:** Silent staleness if Discord adds/renames a region.
- **Suggested fix:** Either resolve regions from the Discord API at startup and
  cache, or leave a dated note that manual sync is accepted. Low priority.

### 4. `actions` layer untested  ·  #6  ·  med
- **Status:** `complete — 2503adc1`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Reconciled (2026-08-06):** the marker was left at `in-review` after the human
  committed the task as `2503adc1` ("Add test for temp voice actions"). Verified
  against the tree, not the record: the commit carries
  `bot-modules/temp-voice/tests/actions_authz.rs` (315 lines) and the `tokio`
  dev-dep in `bot-modules/temp-voice/Cargo.toml`, and touches no `src/` file —
  matching this note's "pure regression net, no `src/` change" claim.
- **Fix (2026-08-06).** `bot-modules/temp-voice/tests/actions_authz.rs` — 15 tests
  pinning the **gate mapping**: which of the two private guards
  (`actions/mod.rs:15-29`) each of the 11 mutations uses. Owner-gated: `trust`,
  `password`, `transfer`, `delete`. Trusted-gated: `kick`, `privacy`, `rename`,
  `limit`, `bitrate`, `region`. `claim` deliberately uses neither — it is how a
  non-owner takes over an abandoned channel — so only its own first-statement
  `UserIsOwner` invariant is covered. Sole non-test change: a `tokio` dev-dep
  (`macros`, `rt`) on the crate. No `src/` change — **this finding was a missing
  regression net, not a defect; the gate mapping was already correct.**
- **The tests assert the exact `PermissionError` variant, not merely "an error."**
  That is the load-bearing choice. Swapping `require_owner` for `require_trusted`
  on an owner-only action still rejects an *outsider*, so an outsider-only test
  would stay green through a real privilege escalation. Each owner-gated action
  therefore has a **trusted-non-owner** case, which is the only caller that can
  distinguish the two guards.
- **Offline by construction.** Every guard returns before its action's first
  `.await`, so the `Http` (fake-but-well-formed token) and the lazy `PgPool` are
  constructed and never dialled. No `#[sqlx::test]`, no `DATABASE_URL`, no
  network — unlike the `gold-star`/`llamad2` CC-6 harnesses.
- **Verification — guard-removal matrix (CC-6's prescribed substitute for
  fails-before).** A test written against already-correct code cannot fail first,
  so each guard was mutated in turn and the suite re-run: **21 mutations, 21
  caught, 0 survivors.** Both shapes were tried per site — *delete the guard*, and
  *swap it for the other guard* (the escalation shape). Owner sites are caught by
  2 tests each, trusted sites by 1.
  **A correction worth recording:** the first pass scored 21/21 only because it
  counted 10 **compile errors** as catches. They were not — mutating a guard line
  alone leaves the other guard un-imported (or the old one unused), so the crate
  never built and the suite never ran. Those 10 were redone with the `use
  super::…` line fixed too, so the mutant compiled and the tests genuinely
  executed; all 10 were then caught for real. **A build error is the compiler
  rejecting a malformed mutation, not evidence about the test.** Any future
  mutation-testing verification in this workspace should classify build failures
  as *invalid mutants to be repaired*, never as passes.
- **Where:** `src/actions/*` (11 extracted mutations incl. the server-side
  owner/trusted re-checks) vs. `tests/components.rs` (structural button
  assertions only).
- **What:** The security-relevant re-check logic (guessable custom-ids can't
  bypass owner/trusted checks — the M4 design's core claim) has no test.
- **Why it matters:** The permission gate is exactly the thing worth a
  regression net.
- **Suggested fix:** Add `tests/` coverage for `require_owner`/trusted-check
  branches (the M3 `loadout_refresh.rs` permission test is a template).

### 5. `setup` duplicates the dashboard's temp-voice settings  ·  #8  ·  med
- **Status:** `complete — 39fae2dc`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-25):** Removed the duplicate editor and closed the one capability
  gap that removal opened. Following the lfg #4 precedent (`51d8412e`): deleted
  `commands/setup.rs`, dropped the `setup` subcommand from `Command::register()`
  and its match arm (`commands/mod.rs`), and removed the now-orphaned
  `TempVoiceRow::save` (its only caller was `setup`) plus its now-unused
  `PgQueryResult` import — `get`/`get_category`/`get_creator_channel` are
  unchanged. `TempVoiceError::AdministratorRequired`/`IneligibleChannel` stay,
  still used by `panel` and the channel commands.
  **The gap:** unlike lfg's, this `setup` also *created* the "➕ Creator Channel"
  voice channel, which a web form cannot do — the dashboard can only select
  channels that already exist. So the dashboard gained a
  `create_temp_voice_creator_channel` server fn
  (`dashboard/src/server/guild.rs`) that creates the voice channel under a chosen
  category via the bot-token twilight client and points `temp_voice_settings` at
  both, behind the same `guild_admin_context` authz gate as every other settings
  write. The Temp Voice section of the settings page (`ui/pages/guild_settings.rs`)
  now carries a "Create Creator Channel" button beside the existing save form, and
  the settings `Resource` is keyed on that action's `version()` so the new channel
  appears in the pickers immediately (same pattern as `ui/pages/modules.rs`).
  **`.sqlx`:** the change only *removes* one `query!` (the 3-col insert) and adds
  none — the dashboard path reuses the already-cached `TempVoiceSettingsRow::upsert`
  — so the cache was reconciled by deleting just that orphaned entry
  (`query-76409a92…json`); `cargo sqlx prepare` was deliberately **not** run (this
  dev DB is not the empty/freshly-migrated DB required for correct LEFT-JOIN
  nullability inference). **Gates:** `cargo +nightly clippy --workspace
  --all-targets -D warnings` clean, `cargo test` green (0 failures), plus
  `-p dashboard --features ssr` and the wasm/`--features hydrate` check, since this
  touches hydrated UI. No new `#[allow]`/`#[expect]`; no `Cargo.toml` dep change.
  **No test:** a duplication removal with no pure-logic surface (the "regression" is
  that the bot write path no longer exists), and the new server fn needs live
  Discord + DB — `dashboard` has no lib-target test harness (see CC-6). Mirrors the
  lfg #4 / family #3 precedent.
  **Residual:** the dashboard is now the single editor of `temp_voice_settings`;
  `panel` and the live channel mutations stay in-bot as the finding directs.
- **Where:** `src/commands/setup.rs` (writes `temp_voice_settings`).
- **What:** Writes the same row the dashboard now writes via
  `save_temp_voice_settings` — a duplicate editor.
- **Why it matters:** Two write paths to one table; the web form is the better UX.
- **Suggested fix:** Dashboard becomes the single editor; remove/deep-link
  `setup`. **Keep** `panel` and the live channel mutations
  (claim/kick/transfer/limit/rename) in-bot — they need a live voice session. See
  [CC-8](_cross-cutting.md#cc-8).

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lenses: state-machine/authz, concurrency._

### DS-1. `claim`/`transfer` never revoke the previous owner's permission overwrite  ·  Pass 7/8  ·  med
- **Status:** `complete — 08ffb320`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/temp-voice/src/actions/ownership.rs:33-38` (`claim`),
  `:53-58` (`transfer`). Both call `create_permission(.., owner_perms(new))` but
  there is **no** matching `delete_permission` for the old owner (grep confirms
  `delete_permission` only exists in `untrust`/`unblock`).
- **What:** `owner_perms` grants a broad set including `MANAGE_CHANNELS`,
  `MOVE_MEMBERS`, `MUTE_MEMBERS`, `DEAFEN_MEMBERS`, `MANAGE_MESSAGES`
  (`lib.rs:146-173`). After ownership changes, the DB `owner_id` moves to the new
  owner (so the *bot's* `require_owner` commands correctly reject the old owner),
  but the old owner's channel-level permission overwrite is left in place.
- **Failure scenario:** owner A runs `/transfer @B`. B is recorded as owner and
  granted `owner_perms`. A's overwrite is never removed, so A retains
  `MANAGE_CHANNELS`/`MOVE_MEMBERS`/etc. **via Discord's native UI** — A can rename
  or delete the channel, drag members out, or server-mute them, despite no longer
  being the temp-voice owner. Same leak on `claim` (the original owner's overwrite
  persists after someone else claims an abandoned channel).
- **Suggested fix:** in `claim`/`transfer`, `delete_permission` for the previous
  `row.owner_id()` (or downgrade it to member perms) before/after granting the new
  owner. **Confidence: confirmed** (no removal path exists).

### DS-2. `claim` is a racy read-modify-write → stray owner grants  ·  Pass 2  ·  low
- **Status:** `complete — 929a823d`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-22):** `actions::claim`'s read-modify-write (`set_owner` +
  absolute `row.save`) was replaced with a guarded conditional write. New
  `VoiceChannelManager::claim` runs
  `UPDATE voice_channels SET owner_id = $new WHERE id = $c AND owner_id = $expected`
  (concrete impl in `bot/src/bindings/temp_voice/mod.rs`) and returns
  `rows_affected == 1`. The action now grants the new owner's `owner_perms` and
  revokes the previous owner's overwrite **only when that write wins**; a
  same-tick double-claim loses the guard (0 rows) and returns the new
  `TempVoiceError::ClaimFailed` ("claimed by someone else, try again") *before*
  touching Discord permissions — so no stray `owner_perms` overwrite is left on
  the channel. This is the CC-9 pattern (guarded atomic write, not another
  absolute overwrite). New `.sqlx/` entry for the UPDATE. **No regression test:**
  the guard is the SQL `WHERE` clause; the crate has no live-`PgPool`/`Http`
  harness (see [CC-6](_cross-cutting.md#cc-6)), same posture as gold-star/lfg
  DS-1. The existing `tests/ownership.rs` still pins the revoke decision the
  success path relies on. **Transfer note:** `actions::transfer` has the same
  absolute-save shape but is owner-gated (`require_owner`), so it is a
  lower-value residual left for the CC-1 concrete-`PgPool` migration.
- **Where:** `bot-modules/temp-voice/src/actions/ownership.rs:14-41`.
- **What:** `claim` checks `owner_present`, then `set_owner` + absolute `save` +
  `create_permission`. No lock/idempotency (an instance of
  [CC-9](_cross-cutting.md#cc-9), low impact here).
- **Failure scenario:** an abandoned channel is claimed by A and B in the same
  tick. Both pass `!owner_present`, both `create_permission(owner_perms(self))`,
  both `save` (absolute — DB owner ends as whoever writes last, say B). Result: A
  holds `owner_perms` on the channel but is **not** the recorded owner, so A can
  manage the channel via Discord while the bot treats B as owner. Compounds DS-1.
- **Suggested fix:** fold the claim into a single conditional write
  (`UPDATE ... SET owner_id = $new WHERE channel_id = $c AND owner_id = $expected`)
  and only grant perms when it wins. **Confidence: confirmed** for the double-grant
  window; low real-world impact.

## Clean
- #1 Architecture: `actions`/`components`/`commands`/`events` split is clean and
  mirrors LFG conventions; `ModuleComponent`/`ModuleModal` wired in `bot/`.
- #2 Dead code: M4 dropped the stubbed `waiting`/`info` arms; no soft stubs found.
- #3 Async: no blocking I/O; no locks across `.await`.
- #2 (bugs) The 4a extraction fixed the inverted `delete` owner check and the
  `password` option-key mismatch — verified resolved.
