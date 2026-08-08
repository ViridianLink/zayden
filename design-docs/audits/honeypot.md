# Audit: honeypot

_Audited: 2026-08-07 · Commit: `7e75cc79`_

> The last unaudited crate in the workspace. It landed after the 2026-07-17 sweep
> (`c7605e43`, 2026-07-30) and so is absent from the 20-crate coverage list in
> [`README.md`](README.md). This is its first 8-point pass.

## Summary

Structurally the cleanest new crate in the workspace: no generic-manager
indirection (it never had a manager — settings go through the shared
`SettingsRow`/`SettingsStore`), one compile-time `query!`, zero
`#[allow]`/`#[expect]`, and a genuinely good `tests/policy.rs` that pins the
exemption matrix with intent-carrying comments rather than trivia. Nothing here
is CC-1/CC-2/CC-3/CC-5 debt.

This crate is the only code in the workspace that **bans a user with no human
in the loop**, so its failure modes are asymmetric: a false negative lets one
spam message through, a false positive permanently removes a real member. The
residual findings are the duplicated ban-reason literal, the two-editor
duplication of `/honeypot set`, and the untested action path.

## Findings

### 4. Ban reason is a literal duplicated across two crates  ·  #4  ·  low-med
- **Status:** `unclear`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/honeypot/src/message_create.rs:14`
  (`const REASON`) and `bot/src/bindings/honeypot/mod.rs:17`
  (`const HONEYPOT_REASON`) — byte-identical strings, declared independently.
- **What:** The same sentence is the Discord audit-log reason on both the `ban`
  and the `unban`, *and* the `reason` column of the recorded infraction. Two
  constants, two crates, no shared source.
- **Why it matters:** This is the [verify #2](verify.md) class exactly — a value
  duplicated across a module crate and its `bot` binding, where editing one and
  not the other silently desynchronises the Discord audit log from the DB record
  with no compile error and no test to catch it. The blast radius is smaller
  than `VERIFIED_ROLE`'s (a mismatched string misleads a moderator reading
  `/logs` against the audit log rather than breaking a feature), which is why
  this is low-med rather than med.
- **Suggested fix:** One owner. Export the constant from the `honeypot` crate
  (it already `pub use`s its API surface in `lib.rs:7-10`) and have
  `bot/src/bindings/honeypot/mod.rs` use it. `HONEYPOT_MODERATOR` and
  `HONEYPOT_POINTS` are correctly binding-local — they describe how the *bot*
  records the hit, not what the trap does — so leave them.

### 5. `/honeypot set` duplicates the dashboard's honeypot form  ·  #8  ·  med
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Ruling (2026-08-08): keep both surfaces, converge the write.** The owner
  chose the reaction-roles #3 shape over this finding's own recommendation
  (retire `set`/`disable`) and over deleting the command outright. Recording the
  divergence because the finding argued the other way: the audit reasoned from
  "honeypot has no live-tweak field, so all of it is admin setup", which is the
  music #3 lens. That lens ranks *when a field is edited*; it does not weigh
  **whether an admin can reach the web at all** to turn off a feature that
  auto-bans. Arming is setup, but *dis*arming is incident response — and the
  in-Discord path to it is worth keeping even though it duplicates a form.
- **Fix (2026-08-08).** New `bot-modules/honeypot/src/settings.rs` is the single
  owner. `HoneypotConfig` carries the configuration in domain types
  (`Option<ChannelId>` / `bool` / `Option<RoleId>`), `HoneypotSettings::{get,
  arm, disarm, save}` are the operations, and both editors now call them:
  `/honeypot set|disable|status` (`commands/mod.rs:89`, `:115`, `:135`) and the
  dashboard's `save_honeypot_settings` (`dashboard/src/server/guild.rs:383`).
  Neither caller touches `HoneypotSettingsRow`'s columns any more, so the
  snowflake↔column mapping exists once. The dashboard gained a `honeypot`
  optional dep, gated into its `ssr` feature beside `ticket`/`reaction-roles`.
- **The dedupe uncovered a real defect, which is what made this more than
  hygiene.** The two writers did not merely duplicate the write — they
  *disagreed*. The dashboard parsed its form fields with
  `parse_id` (`s.trim().parse().ok()`), which maps **both** "the admin cleared
  this field" and "this field is garbage" to `None`. So a garbled `channel_id`
  **silently disarmed the trap and reported success** — an anti-spam feature
  failing to "off" with a green checkmark. The shared owner's `from_form` splits
  those: blank clears, malformed is `HoneypotError::InvalidSnowflake`. It also
  parses as `u64` rather than `i64`, so a negative id is rejected instead of
  stored. **A CC-8 de-duplication is not always behaviour-neutral** — when two
  writers exist, check whether they *agree* before assuming the only prize is
  one fewer code path.
- **Why `arm`/`disarm` stay field-scoped.** They write `channel_id` only, and
  two tests pin that. If Discord wrote a whole `HoneypotConfig` it would clobber
  the exemptions the dashboard owns — recreating the divergence one layer up.
  That asymmetry (Discord edits one field, the web owns the form) is precisely
  what lets both editors coexist, so it is the invariant to guard, not an
  implementation detail.
- **Verification.** `bot-modules/honeypot/tests/settings.rs`, 10 tests, offline
  (`HoneypotConfig` is a pure value type — no `DATABASE_URL`). Fails-before
  established by mutation, since the API is new and could not fail against the
  old tree:
  - Reverting `parse_optional_id` to the old `.ok()` semantics → **4 of 10 fail**,
    and the failure output is the defect itself: `HoneypotConfig { channel_id:
    None, … }` for input `"not-a-snowflake"`.
  - Making `arm_row` write the whole config → **1 fails**
    (`arming_preserves_the_exemption_policy`).
  - Both mutants were checked to *compile* first — per temp-voice #4's lesson, a
    build error is an invalid mutant, not a catch. The first attempt at mutant 1
    failed `-D unused-imports` and was repaired before it counted.
- **Gates:** `cargo +nightly clippy --workspace --all-targets -- -D warnings`
  exit 0, no new `#[allow]`/`#[expect]` (the one lint it raised,
  `missing_const_for_fn` on a test helper, was fixed by making the fn `const`);
  `cargo test --workspace --no-fail-fast` **674 passed / 0 failed / 7 ignored**
  (all 7 are pre-existing live-API tests); `-p dashboard --features ssr` and
  `--target wasm32-unknown-unknown --features hydrate` both clean;
  `cargo +nightly fmt` applied; `cargo machete` clean. **No `.sqlx` delta** — the
  change adds no `query!`, and `git status .sqlx` is empty.
- **Residual:** finding #8 (`GUARD.forget` clearing the wrong cache) sits in the
  two command handlers this task touched and was deliberately **not** folded in —
  it is its own finding and the `forget` calls are unchanged. Also note the
  dashboard still cannot reach `GUARD` at all (separate process), so a web-side
  exemption edit remains subject to the 5-minute `facts` TTL; that is #8's
  territory, recorded there as intended behaviour.
- **Where:** `bot-modules/honeypot/src/commands/mod.rs:82-128` (`set`) and
  `:130-154` (`disable`) vs. `dashboard/src/server/guild.rs:383-402`
  (`save_honeypot_settings`) + `dashboard/src/ui/pages/guild_settings.rs:363-380`
- **What:** Two editors, one table — the [CC-8](_cross-cutting.md#cc-8) shape
  that closed lfg #4 and temp-voice #5. The dashboard form writes all three
  columns (`channel_id`, `exempt_admins`, `exempt_role_id`); `/honeypot set`
  writes `channel_id` and `/honeypot disable` clears it. The split is already
  half-acknowledged in the product copy: `/honeypot status` tells the admin
  *"Change the exemptions from the dashboard"* (`commands/mod.rs:177`), so
  the command is already the lesser of the two editors.
- **Why it matters:** Arming a trap that auto-bans is one-shot admin
  configuration — the exact profile the CC-8 heuristic sends to the web. It also
  carries the standard divergence risk: `SettingsStore::update` is a
  read-modify-write over the cached row with an absolute upsert
  (`zayden-app/src/config/settings_store.rs:74-80`), so two editors racing lose
  one edit. (That is last-writer-wins **by design** for settings rows per CC-9's
  2026-07-29 re-sweep, so it is not a defect on its own — but it is one more
  reason not to keep two writers.)
- **Suggested fix:** Apply the music #3 lens — per **field**, not per command —
  and note it lands differently here than it did for music. Music kept the
  fields tweaked *during playback*; honeypot has no live-tweak field at all, so
  the whole of `set`/`disable` is admin setup and belongs on the web. The
  counter-argument is the reaction-roles #3 one: Discord's channel picker beats
  a web field for choosing the decoy channel — but unlike the emoji picker, the
  dashboard already ships a real channel picker on this exact form
  (`<ChannelSelect label="Honeypot Channel" … kinds=TEXT_KINDS/>`,
  `dashboard/src/ui/pages/guild_settings.rs:374-380`), not a text field, so that
  argument does not carry. Recommend: keep **`/honeypot status`** in-bot (a read-only
  echo, mirroring the loadout `refresh` pattern) and retire `set`/`disable`.
  **This is a direction finding and a product call, not a defect — it needs the
  owner's ruling before any code moves.**

### 6. `HoneypotHit.channel_id` is constructed and never read  ·  #2  ·  low
- **Status:** `unclear`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/honeypot/src/message_create.rs:25` (field),
  `:99` (populated); sole consumer is
  `bot/src/bindings/honeypot/mod.rs:46-59`, which reads `guild_id`, `user_id`
  and `username` only.
- **What:** A dead field on a live struct.
- **Why it matters:** Small, but it is the [gambling #2b](gambling.md) shape and
  worth naming as such: `pub` field on a `pub` struct in a `pub`-re-exported
  module means rustc treats it as reachable API, so `dead_code` never fires and
  the `-D warnings` gate cannot see it. The same blind spot that hid
  `WEAPON_CRATE`. The 2026-07-29 reserved-const policy does **not** shield this
  one — that policy covers reserved *catalogue* items (planned shop features);
  this is a struct field with no feature behind it.
- **Suggested fix:** Delete the field and its initialiser, or consume it — the
  recorded infraction arguably *should* say which channel the hit came from,
  which would be the better end state and makes the finding a two-line feature
  rather than a deletion. Either resolution is fine; leaving it as-is is not.

### 7. Only `policy.rs` is tested — the action path and the authz gate are not  ·  #6  ·  med
- **Status:** `unclear`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/honeypot/tests/{policy,guard}.rs`; untested:
  `message_create.rs`, `commands/mod.rs:71-80` (`require_manage_guild`).
- **What:** The existing test file is good and should be said so plainly — it
  pins the exemption matrix, and its comments record *why* each case is the way
  it is ("The user chose 'guild owner only by default'…", `:75-77`), which is
  the opposite of the trivia checklist #6 warns about. It covers the pure
  decision function. It does not cover the code that acts on that decision.
- **Why it matters:** The untested surface is where this audit's `med` findings
  survived. `require_manage_guild` is separately notable: it is an
  **authz gate**, the class [temp-voice #4](temp-voice.md) was ranked `med` for,
  and it is a hand-rolled permission check
  (`interaction.member.permissions.is_some_and(manage_guild)`) sitting *behind*
  a `default_member_permissions(MANAGE_GUILD)` declaration — belt-and-braces
  that nothing verifies still agree.
- **Suggested fix:** Take it in the order the seams allow, not all at once.
  (a) `require_manage_guild` needs a `CommandInteraction`, which is the same
  fabrication problem that ruled [verify #1](verify.md) untestable — check
  whether temp-voice's `tests/actions_authz.rs` harness (`2503adc1`) generalises
  before assuming it is untestable. (b) `message_create`'s ban/unban sequencing
  needs an injectable HTTP seam that does not exist today; do **not** invent one
  speculatively — let the coverage follow a refactor rather than drive it.

### 8. `GUARD.forget` invalidates a cache the settings change cannot affect  ·  #2  ·  low
- **Status:** `unclear`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/honeypot/src/commands/mod.rs:112` and `:144`, against
  `bot-modules/honeypot/src/guard.rs:74-76`
- **What:** `HoneypotGuard` holds two independent caches: `facts` (guild owner +
  role→permission map, 5 min TTL) and `recent` (the per-offender action guard,
  1 min TTL). `forget` clears **`facts` only**. Both `/honeypot set` and
  `/honeypot disable` call it after writing `channel_id` — but `channel_id`
  lives in the settings row, whose cache is invalidated by the
  `honeypot_settings_notify` trigger (`migrations/0022_honeypot.up.sql:8-12`)
  feeding `SettingsStore::spawn_invalidator`
  (`zayden-app/src/config/registry.rs:63`), not by `forget`. So the call
  discards a correct cache for no reason and does not touch the one a re-arm
  might plausibly want cleared (`recent`).
- **Why it matters:** Harmless at runtime — dropping `facts` costs one extra
  `to_partial_guild` — but it reads as invalidation-on-config-change and it is
  not, which is the kind of thing a later reader trusts. It also hides the
  crate's one genuine staleness window, worth recording explicitly since it is
  *not* a defect: `facts` is 5-minute TTL and the dashboard (a separate process
  from the bot) has no way to reach `GUARD` at all, so in an `exempt_admins`
  guild a **newly promoted** admin is un-exempt for up to 5 minutes. That is a
  deliberate, bounded trade and should be commented as one rather than fixed.
- **Suggested fix:** Either drop the two `forget` calls, or — if the intent was
  to let an admin re-arm and immediately re-test the trap on themselves — make
  it clear `recent` instead, which is what would actually have that effect. Add
  a line to `guard.rs` recording the 5-minute exemption staleness as intended.

### 9. `PURGE_WINDOW` is a hardcoded 24 h  ·  #5  ·  low
- **Status:** `unclear`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/honeypot/src/message_create.rs:13`, applied at `:72`
- **What:** The ban's `delete_message_seconds` is a compile-time constant.
- **Why it matters:** Checklist #5 — it is the crate's most destructive
  parameter (it deletes 24 h of the target's messages *server-wide*, not just in
  the decoy channel), and it is the one honeypot behaviour an admin cannot
  adjust, while the two far less consequential exemption flags are both DB-backed
  and editable from the dashboard. A guild that wants the trap but not the
  server-wide purge has no setting for it.
- **Suggested fix:** A nullable `purge_seconds` column on `honeypot_settings`
  with the current 24 h as the default, surfaced on the existing dashboard form.
  Clamp to Discord's 0–604800 range at the boundary. Low priority — the current
  value is a reasonable default and no one has asked; recorded because #5 asks
  for it and because it is cheap to add alongside #5's dashboard work.

## Clean

- **#1 Architecture & layering** — clean, and notably so for a crate this new.
  Module tree is `commands` / `error` / `guard` / `policy` / `message_create`
  with `lib.rs` re-exporting a deliberate surface (`lib.rs:7-10`). No DB-generic
  `async_trait` manager ([CC-1](_cross-cutting.md#cc-1)) — it has no manager at
  all, correctly: guild config goes through the shared
  `SettingsRow`/`SettingsStore` pattern
  (`zayden-app/src/config/tables/honeypot.rs`), which is the convention CC-8
  names as the right backend. Command routing is a manual `ModuleCommand` impl
  in `bot/src/bindings/honeypot/mod.rs` — no `poise`. The crate's only SQL is
  no direct SQL at all, so no [CC-5](_cross-cutting.md#cc-5) runtime-SQL bypass.
- **#2 Dead code & stubs** — no `todo!()`/`unimplemented!()`, no orphaned
  modules (all five are `mod`-declared in `lib.rs`, unlike the
  [bot DS-2](bot.md) / bot #4 tree), no commented-out registry entries, no soft
  stubs. `HoneypotError::Internal` looks unreferenced but is reachable via
  `From<HandlerError>` (`error.rs:45-47`). Findings #6 and #8 are the only #2
  items, and both are small.
- **#3 Async correctness** — no blocking `std::fs`, `std::thread::sleep`, or
  CPU-bound loop on an async path. No `unwrap()`/`expect()` on fallible I/O:
  `purge_seconds` uses `try_from(…).unwrap_or(u32::MAX)` (`message_create.rs:17`)
  and `expect_channel` is a no-panic id retype, not an `Option::expect`
  (verified against `serenity/src/model/channel/mod.rs:64` — it is
  `ChannelId::new(self.get())`). No lock held across an `.await`: `moka` caches
  take no user-visible guard, and there is no `Mutex`/`RwLock` in the crate, so
  the [destiny2 DS-1](destiny2.md) tokio-lock class does not apply here. The
  guard-release paths are correct — every early return after a successful
  `claim` releases it.
- **#4 Stringly typing & magic values** — subcommand dispatch is a `match` on
  `&str` (`commands/mod.rs:62-68`), which is the workspace-standard shape for
  slash subcommands (`parse_subcommand` returns the name) and *not* the
  [CC-7](_cross-cutting.md#cc-7) `custom_id` class — there are no components in
  this crate, so there is no producer/consumer pair to desynchronise. It also
  has a real `_ =>` error arm rather than a silent fall-through. Ids are all
  DB-backed (`channel_id`, `exempt_role_id`); no hardcoded guild/role/channel
  constants — contrast [verify #2](verify.md). Finding #4 is the one duplicated
  literal.
- **#5 Data placement** — the exemption policy is correctly in the DB and
  editable at runtime; the crate holds no `const` lookup table. Finding #9 is
  the single constant that arguably belongs in config.
- **#6 Tests** — placed correctly in `tests/` with no inline `#[cfg(test)]`
  ([CC-2](_cross-cutting.md#cc-2) clean). Offline and pure, so they need no
  `DATABASE_URL` — the `#[sqlx::test]` harness does not apply. Quality of what
  exists is good (see #7); coverage of what does not is finding #7.
- **#7 Lint hygiene** — **zero** `#[allow]`/`#[expect]` in the crate, source and
  tests. It is one of the few crates that never contributed to
  [CC-3](_cross-cutting.md#cc-3)'s inventory. All seven declared dependencies
  are used (`zayden-core`, `zayden-app`, `moka`, `serenity`, `sqlx`,
  `thiserror`, `tracing`), and `[lints] workspace = true` is inherited.
- **#8 Dashboard suitability** — already largely done and done the right way:
  the settings live on the existing guild-settings page, the module is
  registered in the dashboard's `MODULES` list
  (`dashboard/src/server/modules.rs:106-109`), and the exemption fields are
  web-only by design. Finding #5 is the residual duplication, not a missing
  page.

## Notes for a re-audit (traced, not findings)

- **The honeypot check runs on every guild message**, before the settings early
  return (`bot/src/handler/message_create.rs:30-37`). It is cheap — a
  `SettingsStore` cache hit, then two integer compares — and the honeypot crate
  itself returns at `message_create.rs:33-44` without touching Discord or the DB
  unless the message is in the decoy channel. Not a hot-path concern.
- **A honeypot error aborts the rest of `message_create`** (`handler`'s `?` at
  `:31` and `:33`), taking levels XP, the AI reply, ticket support and llamad2
  with it. Reachable two ways: the settings `get` at `:31` on a transient DB
  error (affects *every* guild message), or the honeypot's own `to_partial_guild`
  failure (confined to decoy-channel messages). This is a `bot`-crate error
  boundary question, not a honeypot one, and it predates this crate.
- **Bot authors cannot trip the trap**: `handler/message_create.rs:25-28` returns
  early on `msg.author.bot()`, so the [reaction-roles DS-1](reaction-roles.md)
  "handler never skips the bot itself" class does not apply. Worth noting the
  trap therefore does **not** catch a compromised *bot* account, which is a
  deliberate consequence of a filter that sits above this crate.
- **`msg.member` is trusted for the role list** (`message_create.rs:54`, falling
  back to `&[]`). Discord populates the partial member object on guild
  `MESSAGE_CREATE`, so the empty fallback should be unreachable; if it ever were
  not, the failure is a **false positive** (no roles → no exemption → ban). Not
  raised as a finding because no reachable path was found, but it is the one
  place in the crate where a missing input fails toward banning rather than
  toward inaction. Re-check if serenity's `Message::member` semantics change.
- **The `everyone` role is handled correctly** (`policy.rs:35`) — its id is the
  guild id and it is not in the member's own role list. `tests/policy.rs:98-105`
  pins this, and getting it wrong would soft-ban staff in a guild that grants
  permissions server-wide. Recorded because it is the subtlest correct thing in
  the crate.
