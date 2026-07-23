# Audit: family

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Clean `commands/` + `components/` + manager + `relationships.rs` structure for a
relationship-graph feature (~1.5k LOC). Carries the DB-generic `async_trait`
pattern (CC-1) and — notably — its *only* tests are **inline** `#[cfg(test)]`
modules (CC-2), which is both a convention violation and effectively no
`tests/`-visible coverage.

## Findings

### 1. DB-generic `async_trait` manager  ·  #1  ·  high
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-23):** CC-1 concrete-`PgPool` migration (fifth module after the
  `gold-star`/`levels`/`reaction-roles`/`suggestions` pilots). Dropped the
  `#[async_trait] trait FamilyManager<Db: Database>` and its lone
  `impl … for FamilyTable` binding. The SQL now lives in the crate as concrete
  `PgPool` associated functions (`family_manager.rs` renamed to `manager.rs`):
  `FamilyRow::{get,save,tree,reset,remove_partner,remove_block}` (plus the
  private `build_tree` recursion and the `ensure_family_member` helper, moved
  from the binding) and `FamilySettings::get`. Every command/component `run`
  lost its `<Db, Manager>` generics and now takes `&PgPool` directly; the
  `bot/src/bindings/family.rs` shims drop their `::<Postgres, FamilyTable>`
  turbofish and the `FamilyTable`/`Postgres`/`PgPool`/`FamilyManager` imports.
  `bot/src/bindings/family` is reduced to `register` + the
  `ModuleCommand`/`ModuleComponent` shims (`FamilyTable` deleted). Removed the
  now-unused `async-trait` dependency (`cargo machete` clean; the crate has no
  other `async_trait` use). **Behaviour-preserving:** every `query!` string was
  moved byte-identically, so the existing `.sqlx` cache entries are reused
  unchanged (`git status .sqlx` clean — no regeneration needed). Added
  `tests/manager.rs` pinning the migrated types' pure logic (relationship
  classification, block/partner-limit/adopted predicates, settings clamp).
  Only the larger generic managers — `gambling`, `lfg`, `temp-voice`, plus the
  `zayden-core` traits — now remain on CC-1.
- **Where:** `src/family_manager.rs`, all `commands/*`, `components/*`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1).

### 2. Tests are inline `#[cfg(test)]` in `src/`  ·  #6  ·  med
- **Where:** `src/family_manager.rs:125`,
  `src/commands/information/siblings.rs:90`.
- **What:** The relationship-graph logic *is* tested, but inline in `src/` —
  violating the `tests/`-only convention (CC-2) and leaving the crate with no
  `tests/` directory.
- **Why it matters:** Convention drift, and the relationship-resolution logic
  (siblings/parents/children graph walks) is high-value coverage that should be
  visible in `tests/`.
- **Suggested fix:** Relocate both modules to `tests/`, exposing the minimum
  `pub(crate)` surface needed. See [CC-2](_cross-cutting.md#cc-2).

### 3. `family_settings` (per-guild config) belongs on the dashboard  ·  #8  ·  low
- **Status:** `complete — 8bfa50fd`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-22):** Added the dashboard editor surface. New
  `FamilySettingsRow` (`SettingsRow`) in
  `zayden-app/src/config/tables/family.rs` (mirrors lfg/temp-voice: `select`
  + guarded `ON CONFLICT` `upsert` via compile-time `query_as!`), registered in
  the `SettingsRegistry` (`family` store + cache invalidator). Dashboard now
  reads it in `get_guild_settings` and writes it via a new `save_family_settings`
  server fn (floors `max_partners` at 1), with a "Family" section on the settings
  page. Bot-side enforcement (`bindings/family.rs`) is unchanged — the website is
  now the single editor. **Residual:** the `family` slash-command has no editor to
  remove (there never was one); the read-only `max_partners` was previously
  un-editable at runtime and now is.
- **Where:** `family_settings` table (added in migration `0015_family_guild_scope`,
  guild-scope design change 2026-07-22); currently read bot-side only, with **no**
  editor surface.
- **What:** The guild-scope change made family per-guild and introduced
  `family_settings (guild_id, max_partners, …)`, but deliberately shipped
  **bot-side only** (DB + enforcement read). There is no way for a guild admin to
  change `max_partners` yet.
- **Why it matters:** Per [CC-8](_cross-cutting.md#cc-8), per-guild config is the
  dashboard's domain (mirrors `save_lfg_settings` / `save_temp_voice_settings`
  etc.). A `save_family_settings` server fn + a settings-page section is the
  intended editor; a bot slash-command editor is the weaker fit.
- **Suggested fix:** Add a `FamilySettingsRow` (`SettingsRow`) in
  `zayden-app/src/config/tables/family.rs`, register it, and build the dashboard
  `save_family_settings` server fn + settings section. Follow-up to the
  guild-scope change.

## Clean
- #1 Architecture: clean command/component/manager/relationships split.
- #1 DB access: concrete impl uses compile-time macros (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
- #7 Lint: one `#[expect]` at `commands/tree.rs:71` (CC-3).

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lenses: silent-failure, state-machine/invariant, concurrency._

### DS-1. `/block` is never enforced and `/unblock` never persists → entire block feature is inert  ·  Pass 1+7  ·  med
- **Status:** `complete — d2d05898`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot/src/bindings/family.rs:206-277` (`FamilyTable::save` — additive
  `INSERT … ON CONFLICT DO NOTHING`, no `DELETE` for blocks);
  `bot-modules/family/src/commands/block.rs:53-79` (`Unblock::run` →
  `remove_blocked` + `save`); `bot-modules/family/src/commands/adopt.rs:19-63`
  and `commands/marry.rs:21-66` (neither reads `blocked_ids`). `blocked_ids` is
  written by `add_blocked`/`remove_blocked` and loaded by `row`, but the grep
  (`blocked_ids` usages) shows **no read site** that gates a proposal.
- **What:** Two defects compound into a fully non-functional feature the UI claims
  works:
  1. **Block is a no-op.** `/block` inserts a `family_blocks` row and replies
     "User blocked.", but `adopt`/`marry` never consult `blocked_ids`, so a
     blocked user can still adopt/marry the blocker. The command's own
     description ("Blocks a user from being able to adopt/marry/etc you") is
     never honoured.
  2. **Unblock cannot remove the row.** `Unblock::run` mutates the in-memory
     `blocked_ids` vector via `remove_blocked`, then calls `save` — but `save`
     only issues `INSERT … ON CONFLICT DO NOTHING` per remaining id and has **no**
     `DELETE`. The removed block is never deleted from `family_blocks`; the
     command replies "User unblocked." while the DB is unchanged. (Contrast
     `remove_partner`, which is a dedicated `DELETE` — blocks have no equivalent.)
- **Failure scenario:** User A runs `/block @B` → `family_blocks (A,B)` inserted,
  reply "User blocked." B then runs `/marry @A` or `/adopt @A` → succeeds, because
  no code reads `blocked_ids`. A runs `/unblock @B` → reply "User unblocked." but
  `SELECT * FROM family_blocks WHERE user_id = A` still returns `(A,B)`. Every
  subsequent `/block`/`/unblock` of anyone re-persists the stale row (additive
  save re-inserts all `blocked_ids`).
- **Why it matters:** A user-facing safety feature (blocking unwanted
  adopt/marry proposals) is silently dead in both directions. Users relying on it
  are unprotected and cannot tell, because both commands report success.
- **Confidence:** confirmed (read all four paths + schema; the enforcement read
  site does not exist).
- **Suggested fix:** (a) Gate `adopt`/`marry` (and their accept handlers) on the
  target's `blocked_ids`. (b) Give blocks a dedicated `DELETE` path
  (`remove_block(pool, user, blocked)`) mirroring `remove_partner`, instead of
  routing removal through the additive `save`. Note the additive `save` is a
  general hazard: **any** future "remove a relationship by mutating the vec + save"
  will silently no-op the same way.

### DS-2. `marry`/`adopt` accept handlers re-run no invariant checks → `MAX_PARTNERS`/already-adopted bypass  ·  Pass 7  ·  low
- **Status:** `complete — 50f551d4`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-22):** Both `accept` handlers now re-validate the invariants
  against the *freshly-read* rows, before the write:
  `components/marry.rs::accept` rejects with `AlreadyRelated` if the pair is
  already related and `MaxPartners` if **either** party is `at_partner_limit()`;
  `components/adopt.rs::accept` rejects with `AlreadyAdopted` if the child
  `is_adopted()` and `AlreadyRelated` if the pair is already related. Two new
  pure guards on `FamilyRow` — `at_partner_limit(max)` / `is_adopted()` —
  encapsulate the checks. **Folded into the guild-scope design change
  (2026-07-22):** the partner cap is no longer a `const`; it is the guild's
  configured `family_settings.max_partners` (default 1), read via
  `Manager::settings(pool, guild_id)` and passed to `at_partner_limit(max)` at
  both propose and accept time. Regression test `tests/invariants.rs` pins the
  guards against the configured cap (incl. a raised cap permitting another
  partner, and negative-cap clamping); they didn't exist pre-fix, so the accept
  handler had nothing to consult.
  **Residual:** this closes the *sequential* accept-both scenario (the second
  accept re-reads the updated row and is rejected). The *same-tick concurrent*
  double-accept is still the [CC-9](_cross-cutting.md#cc-9) read-modify-write
  race (both reads see the stale pre-image); a truly atomic guard needs the
  conditional-write / [CC-1](_cross-cutting.md#cc-1) concrete-`PgPool` migration
  of the additive `save`, out of scope for this low-sev surgical fix.
- **Where:** `bot-modules/family/src/components/marry.rs:8-33` (`accept`),
  `bot-modules/family/src/components/adopt.rs:8-37` (`accept`). The guards
  (`MAX_PARTNERS`, "already adopted", "already related") live only in the
  *command* (`marry.rs:44-63`, `adopt.rs:46-60`), evaluated at proposal time.
- **What:** Between proposal and accept, state can change; the accept handler
  blindly `add_partner`/`add_child` + additive save with no recheck.
- **Failure scenario:** X sends `/marry @Z`; Y sends `/marry @Z` (both pass the
  command-time check because Z has 0 partners). Z clicks accept on both. Each
  accept adds a distinct partner via `ON CONFLICT DO NOTHING` on different pairs,
  so Z ends with 2 partners despite `MAX_PARTNERS = 1`. Same shape for two pending
  adoptions of one free child → child gets two parents.
- **Why it matters:** Invariant (`MAX_PARTNERS = 1`, single-parent adoption) is
  bypassable by anyone with two pending proposals; low severity because it needs
  cooperating/duplicate proposals and only corrupts the social graph, not economy.
- **Confidence:** confirmed (logic traced; no recheck exists).
- **Suggested fix:** Re-validate the invariants inside `accept` within the same
  transaction as the write (or make the write conditional: insert only if the
  partner/parent count is still under the cap), mirroring the CC-9 remediation.
