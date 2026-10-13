# Audit: lfg

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Well-structured (the `actions`/`components`/`commands`/`modals`/`models` split
that temp-voice and others mirror), on concrete `PgPool` with integration tests
under `tests/`. Structurally the best migration reference alongside temp-voice.

## Findings

### 3. `#[expect]` escape-hatches  ·  #7  ·  low
- **Status:** `open`
- **Where:** `src/actions/leave.rs:19`, `src/cron/reminders.rs:20`.
- **What / Why / Fix:** See [CC-3](_cross-cutting.md#cc-3).

### 4. `setup` duplicates the dashboard; `tags` CRUD belongs on the web  ·  #8  ·  med
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-24):** Removed the duplicate editor. The dashboard's
  `save_lfg_settings` (`dashboard/src/server/guild.rs:229`) already writes a
  **superset** of the same `lfg_settings` row (channel + role + scheduled-thread),
  so `/lfg setup` was a second, weaker editor of one table — the CC-8 "two editors,
  one table" divergence. Deleted `commands/setup.rs`, dropped the `setup`
  subcommand from `Command::register()` + its match arm (`commands/mod.rs`), and
  removed the now-dead `GuildRow::insert` (its only caller was `setup`) plus its
  now-unused imports (`GenericChannelId`/`RoleId`/`PgQueryResult`) — `GuildRow::get`
  (read by the create modal) is unchanged. Repointed the `MissingSetup` user
  message (`error.rs`) from "run `/lfg setup`" to "configure … in the web
  dashboard" so the guidance matches the single remaining editor. **`.sqlx`:** the
  fix only *removes* one `query!` (the 3-col insert), adds/changes none, so the
  offline cache was reconciled by deleting just that orphaned entry
  (`query-a5658c9c…json`); `cargo sqlx prepare` was deliberately **not** run (this
  dev DB is not the empty/freshly-migrated DB required for correct LEFT-JOIN
  nullability inference — regenerating would risk drifting unrelated entries).
  Dashboard `save_lfg_settings` remains the single write path; live post
  create/join/leave/kick stay in-bot. **No test:** this is a structural
  duplication removal with no pure-logic surface — the "regression" is that the
  bot write path no longer exists (verified by the `setup` symbol and its SQL
  being gone + a clean workspace build); a fails-before/passes-after test isn't
  feasible (mirrors lfg DS-1 / family #3). No new `#[allow]`/`#[expect]`.
  **Residual:** the finding's second half — moving `tags` reference-data CRUD to a
  dashboard page — is a separate UX migration (not a duplication defect, since no
  dashboard `tags` editor exists yet) and is left as a follow-up; spin into its own
  finding if pursued.
- **Where:** `src/commands/setup.rs` (writes `lfg_settings` via `Manager::insert`),
  `src/commands/tags.rs`.
- **What:** `setup` writes the exact `lfg_settings` row the dashboard already
  writes via `save_lfg_settings` — an active duplicate editor. `tags` is admin
  CRUD of reference data, a natural web form.
- **Why it matters:** Two write paths to one table diverge over time; a one-shot
  config command is a worse form than the settings page that already exists.
- **Suggested fix:** Make the dashboard the single editor; remove `setup` or
  reduce it to a deep-link. Add a tags page. Keep create/join/leave/kick (live
  post interaction) in-bot. See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: clean `actions`/`components`/`commands`/`modals`/`models`
  layering; `ModuleComponent`/`ModuleModal` wired in `bot/`.
- #1 DB access: concrete impls use compile-time macros (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
