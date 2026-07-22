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
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
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
