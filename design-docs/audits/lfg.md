# Audit: lfg

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Well-structured (the `actions`/`components`/`commands`/`modals`/`models` split
that temp-voice and others mirror), but carries the DB-generic `async_trait`
pattern throughout (CC-1) and ships **zero** `tests/` despite ~3.3k LOC of post
lifecycle, slot/alternate bookkeeping, and reminder-cron logic. Structurally the
best migration reference alongside temp-voice.

## Findings

### 1. DB-generic `async_trait` managers  ·  #1  ·  high
- **Status:** `complete — 240b47e5`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-24):** CC-1 concrete-`PgPool` migration (seventh module, after the
  `gold-star`/`levels`/`reaction-roles`/`suggestions`/`family`/`temp-voice` pilots).
  Dropped every generic manager trait — `PostManager<Db>`, `Savable<Db, T>`,
  `TimezoneManager<Db>`, `GuildManager<Db>`, `SetupManager<Db>`, `JoinedManager<Db>`,
  `EditManager<Db>` — and the `#[async_trait]` on each. The SQL now lives in the crate
  as concrete `&PgPool` associated functions: `PostRow::{exists,fetch_owner,get,join,
  leave,edit,delete,save}`, `GuildRow::{get,insert}`, `UserSettings::{get,save}` (the
  new home for the ex-`TimezoneManager` user-settings SQL), `JoinedRow::upcoming`, and
  `EditRow::get`. The six `PostManager` `query_file!` SQL files moved from
  `bot/sql/lfg/PostManager/` into the crate at `bot-modules/lfg/sql/PostManager/`
  (byte-identical → offline cache reused). Every command/component/action/modal/event/
  cron `fn` lost its `<Db, Manager…>` generics and now takes `&PgPool`; the cron/modal/
  event paths that schedule reminders keep a single `Data: CronJobData<Postgres>` type
  param (the zayden-core `CronJob<Db>` generic is its own CC-1 item, left untouched) and
  are pinned to `Postgres`. `bot/src/bindings/lfg/{mod,slash_command}.rs` collapse to the
  `ModuleComponent`/`ModuleModal`/`ModuleCommand` wiring only — `PostTable`/`UsersTable`
  and all trait impls deleted, turbofish dropped; `bot/src/handler/{guild_create,
  thread_delete}.rs` drop their `::<…, GuildTable, PostTable>` turbofish. Removed the
  now-unused `async-trait` dependency (`cargo machete` clean). **Behaviour-preserving:**
  every `query!`/`query_as!`/`query_scalar!`/`query_file!` string was moved
  byte-identically, so the whole workspace compiles under `SQLX_OFFLINE=true` with the
  existing cache and `git status .sqlx` is clean (no regeneration needed). One naming
  change: the DB owner lookup is `PostRow::fetch_owner` (the inherent `PostRow::owner`
  instance accessor already exists). Only `gambling` and the `zayden-core` traits now
  remain on CC-1.
- **Where:** `src/guild_manager.rs`, `src/models/{post,timezone_manager,mod}.rs`,
  all `commands/*`, `components/*`, `actions/*`, `modals/*`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1).

### 2. No integration tests  ·  #6  ·  high
- **Status:** `complete — f148a563`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-27):** Added **36 integration tests** across four new `tests/`
  files, covering the crate's DB-free logic (the `actions`/`components` layer is
  `PgPool` + `Http` bound and stays uncovered — see **Residual**):
  - `tests/fireteam.rs` (9) — the slot/alternate accounting `PostRow::join`'s
    rollback guard is built on: `is_full` flips **exactly** at capacity (5/6 → 6/6),
    stays true past it, alternates never consume a slot, an emptied fireteam is not
    full, and `fireteam_len`'s `try_from(...).unwrap_or(i16::MAX)` saturates high
    rather than wrapping negative. One test pins the guard predicate itself
    (`fireteam_len() > fireteam_size()`) at both boundaries, so an off-by-one that
    would re-open **DS-1** fails here.
  - `tests/post_row.rs` (9) — `PostBuilder` ⇄ `PostRow`: builder seeds the owner
    into slot 1, `build()` carries every field, the `PostRow → PostBuilder → PostRow`
    round trip the edit/copy components perform is lossless (roster included), and
    the zone-normalising-but-instant-preserving `From<PostRow>` behaviour is pinned.
    Two tests assert the crate's **two `TemplateInfo` impls agree field-for-field**,
    so the pre-save (builder) and post-save (row) embeds cannot drift.
  - `tests/templates.rs` (10) — the post's rendered surface: `main_row`/`settings_row`
    emit **exactly** the eight `custom_id`s `bot/src/bindings/lfg/mod.rs` routes on
    with `IdMatch::Exact` (a rename on either side otherwise compiles and silently
    deadens the button), all `lfg_`-namespaced and unique; plus the `Joined: n/size`
    counter, member/alternate mentions, the conditional `Alternatives` and
    `Description` fields, `Event Thread` only on the scheduled-message embed, and the
    three `Announcement` strings.
  - `tests/user_settings.rs` (8) — `locale_to_timezone`: **every** one of the 31
    mapped locales resolves in the tzdb and none lands on the `UTC` fallback.
    `UserSettings::get` swallows a bad name with `unwrap_or(TimeZone::UTC)`, so a
    typo there is otherwise invisible and silently shifts every scheduled post in
    that locale. Plus `ACTIVITIES` catalog invariants (positive sizes ≤ 6, unique
    names, raids 6 / dungeons 3).
  **No production code changed** — this is a coverage finding, so the tests
  characterise current behaviour rather than failing first; none of the 36 failed on
  first run, i.e. no latent defect surfaced. Added `serde_json` as a **dev**-dependency
  (the existing `temp-voice`/`palworld` idiom for asserting on serialised builders).
  Gate: `cargo +nightly clippy --workspace --all-targets -D warnings` clean,
  `cargo test` green (106 suites, 0 failures), no new `#[allow]`/`#[expect]`. No SQL
  touched → no `.sqlx` delta.
  **Residual:** (a) the `actions`/`components`/`cron` layer still needs a test-pool
  harness (CC-6's standing blocker) — the join/leave *transactions* themselves remain
  uncovered; (b) one defect found while reading, **left unfixed and filed below** as
  DS-2; (c) `cargo machete` is **not** clean workspace-wide — an unused `tokio` in
  `levels`, pre-existing on clean `main` and unrelated to this task (filed as
  [levels #5](levels.md)).
- **Where:** no `tests/` directory.
- **What:** The post lifecycle (join/leave/alternate/kick, slot counting) and the
  reminder cron have no coverage — the highest-value untested logic in the
  workspace by LOC.
- **Why it matters:** Slot/alternate accounting bugs are easy to introduce and
  invisible without tests.
- **Suggested fix:** Add `tests/` for the pure post/slot state transitions
  first. See [CC-6](_cross-cutting.md#cc-6).

### 3. `#[expect]` escape-hatches  ·  #7  ·  low
- **Where:** `src/actions/leave.rs:19`, `src/cron/reminders.rs:20`.
- **What / Why / Fix:** See [CC-3](_cross-cutting.md#cc-3).

### 4. `setup` duplicates the dashboard; `tags` CRUD belongs on the web  ·  #8  ·  med
- **Status:** `complete — 51d8412e` (setup duplication removed; `tags` web-page deferred)            <!-- open | in-progress | in-review | complete | wontfix -->
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

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lens: concurrency/atomicity._

### DS-1. Fireteam capacity race → post overfills past `fireteam_size`  ·  Pass 2  ·  med
- **Status:** `complete — 82f308a2`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-19):** `join` now takes a `SELECT id FROM lfg_posts WHERE id = $1
  FOR UPDATE` row lock (`sql/lfg/PostManager/lock_post.sql`) as the first
  statement of its transaction, before the fireteam `INSERT` and the aggregate
  re-read (`bot/src/bindings/lfg/mod.rs:100-102`). Same-tick joins on one post now
  serialise on that row lock: the second waiter blocks until the first commits,
  then its `post_row` re-read sees the peer's committed insert and the existing
  `fireteam_len() > fireteam_size()` guard correctly rejects the overfill.
- **Test note:** No fails-before/passes-after regression test was added — it is not
  feasible in-workspace. The fix is a pure transaction-serialisation change with no
  new pure-logic surface (the capacity predicate was already correct and is
  unchanged), so it can only be exercised by two concurrent live transactions. The
  `join` impl lives in the `bot` binary (no lib target — see CC-2), unreachable from
  an integration test, and the workspace has no DB test harness (CC-6). Verified
  instead by live compile-time SQL validation + workspace clippy/test gate.
- **Where:** `bot/src/bindings/lfg/mod.rs:89-125` (`join`), SQL
  `sql/lfg/PostManager/join.sql` + `post_row.sql`; command path
  `bot-modules/lfg/src/actions/join.rs:68`,
  `bot-modules/lfg/src/components/join.rs:18`.
- **What:** `join` opens a tx, `INSERT INTO lfg_fireteam …` (a *new row* per
  user), re-reads the aggregated `post_row`, and rejects with `FireteamFull` iff
  `fireteam_len() > fireteam_size()`. Because each join inserts a **distinct**
  `lfg_fireteam` row (different `user_id`), the two transactions do **not**
  contend on any shared row lock — unlike an `UPDATE` on the post row would. Under
  Postgres' default `READ COMMITTED`, each tx's re-read sees its own insert plus
  only *committed* peers, not the concurrent uncommitted insert.
- **Failure scenario:** a 6-slot post currently has 5 members. Users A and B click
  **Join** in the same tick. Tx A inserts A, re-reads → 6 members, `6 > 6` is
  false → commit. Tx B (concurrent) inserts B, re-reads → its snapshot still shows
  5 committed + B = 6, `6 > 6` false → commit. Final fireteam = **7 members**, one
  over `fireteam_size`. N simultaneous clicks against the same pre-image → `5 + N`
  members. The invariant "`fireteam` never exceeds `fireteam_size`" is broken and
  stays broken (later joins then see `>size` and are correctly rejected, so the
  post is stuck over capacity).
- **Suggested fix:** serialize the check by taking a row lock on the post first —
  `SELECT id FROM lfg_posts WHERE id = $1 FOR UPDATE` at the top of the join tx —
  or enforce capacity with a DB constraint/trigger on `lfg_fireteam` count.
  **Confidence: confirmed** (distinct-row inserts + `READ COMMITTED` snapshot;
  the check is `>` on a stale count).

### DS-2. Last member leaving renders an embed field with an **empty value** → Discord 400  ·  Pass 3 (Discord-API correctness)  ·  med
- **Status:** `complete — 46312479`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-27):** Guarded the `Joined` field's value the way its siblings are
  guarded. `templates.rs:109` now substitutes the placeholder `*Empty*` when the
  roster is empty instead of passing `fireteam.join("\n")`'s empty string straight
  into the field. The `Description`/`Alternatives` siblings drop the field entirely
  when empty, which is not an option here — the `Joined: n/size` header is the
  post's capacity readout and must always render — so a placeholder is the
  equivalent guard. Both `thread_embed` and `message_embed` route through the same
  `embed()` helper, so the scheduled-message render (`update_embeds`' `edit_message`,
  which 400s *first* in the failure scenario) is covered by the one change.
  **Tests (fail-before / pass-after, `tests/templates.rs`):**
  `joined_field_survives_the_last_member_leaving` renders a 0-member post and
  asserts the field is named `Joined: 0/6` with a non-empty value;
  `no_embed_field_is_ever_emitted_with_an_empty_value` sweeps the four shapes a post
  degrades through (empty roster, empty roster + alternate, solo, full) across
  **both** embed builders and asserts every field value is 1–1024 chars — pinning
  Discord's limit itself rather than this one field, so a future unguarded field
  fails here too. Both failed on the pre-fix code with exactly the DS-2 symptom
  (`field `Joined: 0/6` has an illegal value length: ""`) and pass after. Added a
  `fields()` helper that reads `(name, value)` off the serialized embed, using
  `Value::get` rather than indexing (the workspace denies `clippy::indexing_slicing`).
  Gate: `cargo +nightly clippy --workspace --all-targets -D warnings` clean,
  `cargo test` green (12/12 in `templates.rs`, 0 failures workspace-wide), no new
  `#[allow]`/`#[expect]`. No SQL and no `Cargo.toml` dep change → no `.sqlx` /
  `machete` delta.
- **Residual:** the **behaviour** question is deliberately untouched — an emptied
  post is still left orphaned (owner included, since `lfg_leave` is not owner-gated),
  it just renders legally now instead of 400ing. Whether `PostRow::leave` should
  delete the post when the last member (or the owner) leaves is a separate
  semantics decision; file it as its own finding if pursued. The `leave` path's
  other half — the DB write committing before the render is attempted, so any
  render error strands a completed mutation — is the general shape, not specific
  to this field.
- **Found:** 2026-07-27, while writing the lfg #2 coverage (not fixed there — one
  finding per task).
- **Where:** `bot-modules/lfg/src/templates.rs:109` (`fireteam_str`) consumed at
  `:126-130` (the `Joined:` field), reached from `src/actions/leave.rs:92-104` and
  `src/components/leave.rs:15-21`.
- **What:** `embed()` unconditionally emits the `Joined: n/size` field with
  `fireteam.join("\n")` as its value. When the fireteam is empty that value is the
  empty string, and Discord requires an embed field `value` of 1–1024 characters —
  the same class as ticket DS-1/DS-2 (a component built past a hard API limit).
  Nothing keeps the roster non-empty: `PostRow::leave` (`models/post.rs:279-306`)
  just deletes the `lfg_fireteam` row, with no "last member" or "owner" guard and
  no cascade to delete the post. The sibling `Alternatives` and `Description`
  fields are both correctly guarded by an `is_empty()` check; `Joined` is not.
- **Failure scenario:** a solo post (owner only, `Joined: 1/6`). The owner clicks
  **Leave** — the button is on every post's main row and is not owner-gated. The
  `DELETE` commits, then an embed is built whose `Joined: 0/6` field has an empty
  value. `Components::leave`'s `edit_response` 400s on it (`Invalid Form Body`); if
  the post also has a scheduled message, `update_embeds`'s `edit_message` 400s first
  — and only `UnknownMessage` is swallowed there, so that error propagates too.
  Either way `leave` returns `Err` and the user sees an interaction failure
  **after** the DB write already landed. The post is left orphaned with an empty
  fireteam, and every subsequent render of it 400s the same way.
- **Suggested fix:** guard the field like its siblings — emit a placeholder
  (`"*Empty*"`) when the fireteam is empty, or skip the member list and keep only
  the `Joined: 0/6` header. Decide separately whether an emptied post should be
  deleted (owner-leaves semantics), which is a behaviour question, not a rendering
  one. `tests/fireteam.rs::an_emptied_fireteam_is_never_full` already pins that the
  capacity arithmetic handles the 0-member state; only the rendering is broken.
  **Confidence: confirmed** (unguarded field + reachable empty roster).

## Clean
- #1 Architecture: clean `actions`/`components`/`commands`/`modals`/`models`
  layering; `ModuleComponent`/`ModuleModal` wired in `bot/`.
- #1 DB access: concrete impls use compile-time macros (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
