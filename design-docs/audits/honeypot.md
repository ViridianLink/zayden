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

The concerns are all in the **action path**, which is the part `tests/policy.rs`
does not reach. This crate is the only code in the workspace that **bans a user
with no human in the loop**, so its failure modes are asymmetric: a false
negative lets one spam message through, a false positive permanently removes a
real member. Two findings sit exactly there — a failed `unban` that leaves a
standing ban with nothing but a log line (#1), and a per-offender guard whose
check-then-act does not actually hold the "act once per offender" invariant its
own comment claims (#2). Both live in the ~40 lines that `tests/policy.rs`
deliberately stops short of.

## Findings

### 1. A failed `unban` leaves a permanent ban, recorded only as a log line  ·  #3  ·  med
- **Status:** `complete — 9b034aa6`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Reconciled (2026-08-07):** the marker was left at `in-review` after the
  human committed the fix as `9b034aa6` ("Add retry mechanism for transient
  failures") — the same commit that landed this audit file. Verified against the
  tree, not the record: `zayden-core/src/retry.rs` declares all five items the
  fix note names, `message_create.rs:21` sets
  `UNBAN_RETRY = RetryBudget::new(3, Duration::from_millis(250))` with the unban
  behind `retry_transient` at `:88`, `HoneypotOutcome` is declared at `:24` and
  carried on `HoneypotHit` at `:35`, and `bot/src/bindings/honeypot/mod.rs:47-52`
  maps `BanStanding` → `InfractionKind::Ban`. `zayden-core/tests/retry.rs` is
  present (226 lines). `bot/src/handler/guild_create.rs` is in the commit's
  stat, confirming the extraction rather than a second copy.
- **Fix (2026-08-07).** Two halves, because the defect had two: the unban was
  not retried, *and* the outcome was recorded as if it had succeeded.
  - **Retry.** `zayden-core/src/retry.rs` is new — `RetryBudget`,
    `status_is_transient`, `is_transient`, `retry`, `retry_transient`. The
    honeypot unban now runs under `retry_transient` with a **3 × 250 ms**
    budget (`message_create.rs`). `bot/src/handler/guild_create.rs` lost its
    private `is_transient` and imports the shared one, so the workspace has one
    owner for *"is this worth retrying"*.
  - **Honest outcome.** `HoneypotOutcome::{SoftBanned, BanStanding}` rides on
    `HoneypotHit`, and `bot/src/bindings/honeypot/mod.rs` maps `BanStanding` to
    `InfractionKind::Ban`. `/logs` now shows what the guild actually has rather
    than what the trap intended. The `Err` arm deliberately does **not** early-
    return: the ban has already landed, and dropping the hit would also drop the
    infraction record — the only durable trace an admin has.
- **One owner for the mechanism, not for the budget.** `guild_create`'s startup
  retry is 4 × 2 s doubling (~14 s), which would be actively harmful here: this
  runs inside a gateway-event handler and the design case is a raid, where every
  in-flight offender holds a task for the duration of its backoff chain. The
  budget is a `RetryBudget` parameter for exactly this reason. Same shape as the
  `ai #1` → `music #2` lesson (one owner for a timeout budget ≠ one budget), and
  worth restating because the pull toward reusing the constant is strong.
- **A coverage gap found the hard way, and the split that shrank it.** The first
  attempt tested `is_transient` end-to-end and **would not compile**: serenity's
  `ErrorResponse` and `DiscordJsonError` are both `#[non_exhaustive]`, so a
  `serenity::Error::Http` cannot be constructed outside serenity — the predicate
  is untestable as one piece. Rather than fake an error or drop the coverage,
  the judgement was split into `status_is_transient(StatusCode)`, which the
  tests do cover (5xx, 429, and the 428/430 boundary either side of it), leaving
  only the destructuring uncovered. **A type you cannot construct is a coverage
  boundary, not a reason to skip the test** — move the decision to a value you
  can build and keep the untestable part trivial enough to read.
- **Verification.** `zayden-core/tests/retry.rs`, 13 tests. Fails-before is by
  mutation rather than by time — `retry` did not exist before, so nothing could
  fail against the old tree; the matrix is in the file's header, and the row
  that matters is *"drop the loop, call `op()` once"* (the literal pre-fix
  behaviour), which `a_transient_failure_is_retried_then_succeeds` catches.
  Backoff doubling is asserted under `#[tokio::test(start_paused = true)]`, so
  the schedule is pinned without the suite waiting 7 real seconds.
  Gates: bacon `clippy-workspace` (`cargo +nightly clippy --workspace
  --all-targets -- -D warnings`) clean, `.bacon-locations` empty;
  `cargo +nightly test --workspace --no-fail-fast` **653 passed / 0 failed /
  7 ignored** (the 7 are pre-existing live-API tests); `cargo +nightly fmt`
  clean; `cargo machete` clean. No new `#[allow]`/`#[expect]`.
  **No `.sqlx` regen:** the `infractions` INSERT is byte-identical — only the
  value bound to `$4` changes — so no `query!` was added, removed or altered.
- **Residual / follow-ups (all pre-existing findings, none introduced here):**
  - The wiring inside `message_create` — choosing the outcome from the retry's
    result — is still untested, and testing it needs an injectable HTTP seam
    that does not exist. That is [#7](#7-only-policyrs-is-tested--the-action-path-and-the-authz-gate-are-not)(c),
    which predicted exactly this and said to let the coverage follow the
    refactor rather than drive it. It still holds: the refactor this task did
    was the *loop*, not the call.
  - `is_transient`'s destructuring arm remains uncovered — see above.
  - [#4](#4-ban-reason-is-a-literal-duplicated-across-two-crates) untouched:
    `REASON` and `HONEYPOT_REASON` are still two constants. A standing ban is
    now recorded as `Ban` with the unchanged honeypot reason string, which is
    accurate; sharpening the wording is #4's job, not this one's.
- **Where:** `bot-modules/honeypot/src/message_create.rs:71-85`
- **What:** The trap is a **soft**-ban: `ban(purge 24h)` then immediately
  `unban`, so the offender's messages are purged server-wide but a recovered
  account can rejoin. The two calls are sequenced with different error handling.
  A failed `ban` propagates (`:74-75`, and releases the guard). A failed `unban`
  only `error!`s (`:78-85`) — the function then falls through to `warn!` and
  returns `Ok(Some(hit))`, so the caller records a `SoftBan` infraction
  (`bot/src/bindings/honeypot/mod.rs:51`) and the handler returns success.
  Nothing retries, and nothing reconciles the standing ban afterwards.
- **Why it matters:** The failure mode is a **permanent ban on a real member**,
  and every artefact of the event actively denies it: the `/honeypot set`
  confirmation promises the account "is banned … and then immediately unbanned,
  so a recovered account can rejoin" (`commands/mod.rs:118-122`), the recorded
  infraction kind is `SoftBan`, and the only trace of the truth is one `error!`
  line. Note *when* this fires: the crate's whole design case is a raid
  (`message_create.rs:48` — "a flood arrives faster than the ban lands"), i.e.
  many ban+unban pairs against Discord's rate limiter in the same seconds, which
  is precisely when a 429/5xx on the second call is most likely. It is also the
  one call in the pair with no compensating action available to the admin, since
  they have no signal that a ban is outstanding.
- **Suggested fix:** Retry the `unban` with bounded backoff on transient errors
  — `bot/src/handler/guild_create.rs:47-87` already has the exact shape in this
  workspace (`COMMAND_SYNC_ATTEMPTS` + `is_transient`, distinguishing 5xx/429
  from a hard rejection); reuse that predicate rather than writing a second one.
  On final failure the outcome must stop claiming to be a soft-ban: escalate to
  `InfractionKind::Ban` (or a distinct kind) so the recorded state matches the
  guild's real state and the admin can find it in `/logs`. Do **not** simply
  propagate the error — the ban has already landed, so an early return would
  lose the infraction record entirely and make the situation less visible, not
  more.

### 2. `HoneypotGuard::claim` is a non-atomic check-then-act → the "act once per offender" invariant does not hold  ·  #3  ·  med
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-08-07).** `claim` is now a single atomic per-key operation:
  `recent.entry(key).or_insert_with(async {}).await.is_fresh()`. `moka` 0.12
  documents the exact guarantee this needs — *"concurrent calls on the same
  not-existing entry are coalesced into one evaluation of the `init` future.
  Only one of the calls evaluates its future (thus returned entry's `is_fresh`
  method returns `true`), and other calls wait"* — so the check and the insert
  happen under that key's lock and unrelated guilds stay uncontended. The
  doc-comment on `claim` now records why it must not go back to a `get`/`insert`
  pair. `release` and `forget` are unchanged.
- **Verification — the race is real and wide, not theoretical.**
  `bot-modules/honeypot/tests/guard.rs` races `RACERS = 16` concurrent claimers
  over `KEYS = 128` independent keys on a `multi_thread` runtime, gated by a
  `tokio::sync::Barrier` so they arrive together, and asserts exactly one winner
  per key. **Fails-before: 221 winners across 128 floods** (worst key: 3) — a
  73 % overshoot, so the window was not a narrow one. **Passes-after: 128/128**,
  re-run 10× consecutively to confirm it is a property and not a lucky
  schedule. The four accompanying tests (second claim refused, `release`
  re-arms, distinct users and distinct guilds do not share a key) pass against
  **both** implementations by design — they pin existing behaviour so the fix
  cannot silently trade the race away for a broken guard.
- **Two notes for whoever writes the next cache test.** (1) The tests run
  against the production `GUARD` static rather than a widened constructor, so
  the fix adds **no** API surface; the cost is that key disjointness becomes the
  test file's invariant, which is why every test owns its own guild id. (2) A
  current-thread runtime cannot reproduce this — `moka`'s `get` usually
  completes without yielding, so `join_all` polls each racer to completion in
  turn and the buggy code passes. The `multi_thread` flavour is load-bearing,
  not decoration.
- **Residual:** the guard's *other* cache (`facts`) is untouched here; its
  5-minute staleness window is finding #8, and remains deliberate.
- **Where:** `bot-modules/honeypot/src/guard.rs:58-68`, claimed at
  `bot-modules/honeypot/src/message_create.rs:48-52`
- **What:** `claim` is `self.recent.get(&key).await` followed by
  `self.recent.insert(key, ()).await` — two independent `moka` operations with
  an `.await` point between them. Two tasks racing on the same key both observe
  `None` and both return `true`. The comment directly above the call site states
  the invariant this is meant to provide: *"A flood arrives faster than the ban
  lands; act once per offender."* It does not provide it.
- **Why it matters:** This is the workspace's [CC-9](_cross-cutting.md) /
  double-submit shape, on a cache rather than a DB row. **Verified at the
  source, not inherited:** serenity spawns a task per gateway event —
  `spawn_named("dispatch::user", …)` in `gateway/client/dispatch.rs:84` — so
  `Handler::dispatch` (`bot/src/handler/mod.rs:125`) runs concurrently with
  itself and a bot posting three messages into the decoy channel in one tick
  runs three `message_create` futures for one `(guild, user)` key. (CC-9 asserts
  the same property citing `bot/src/handler/interaction/mod.rs:168`; that line
  has since moved and no longer shows it. The claim is still true — the spawn
  was always serenity's, not ours.) The consequence is duplicated
  **ban + unban** pairs: N× the Discord API calls at exactly the moment the
  rate limiter is the constraint, N duplicate `SoftBan` rows in the infraction
  log, N audit-log entries — and it widens the window for #1, because each extra
  unban is another chance for one to fail. The guard is the *only* thing
  standing between a flood and N bans; it is the one place in this crate that
  must be atomic.
- **Suggested fix:** `moka` 0.12 provides an atomic per-key upsert — take the
  claim through `recent.entry(key).or_insert_with(…)` (or `get_with`) and treat
  the returned `EntryExt::is_fresh()` as the claim result, so the check and the
  insert are one operation under the key's lock. Do not reach for a `Mutex`
  around the cache; the per-key primitive is what this needs and it keeps
  unrelated guilds uncontended.
- **Test note:** `guard.rs` needs no Discord context and no DB — `HoneypotGuard`
  is constructible and its three methods are pure cache operations, so a
  `tests/guard.rs` spawning N concurrent `claim`s on one key and asserting
  exactly one `true` is a genuine fails-before test. This is the same
  observation as #7 but is the one piece of it that can be written first.

### 3. Dashboard `save_honeypot_settings` can violate the `guilds` FK; the bot command seeds the row, the web path does not  ·  #1  ·  med
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/honeypot/src/commands/mod.rs:97-102` (seeds) vs.
  `dashboard/src/server/guild.rs:383-402` (does not);
  `migrations/0022_honeypot.up.sql:2` (`guild_id … REFERENCES guilds(id)`)
- **What:** `honeypot_settings.guild_id` is an FK to `guilds(id)`. `/honeypot
  set` inserts the parent row first (`INSERT INTO guilds (id) … ON CONFLICT DO
  NOTHING`) — the only module command in the crate that touches SQL directly,
  and evidence the author hit this. The dashboard's `save_honeypot_settings`
  goes straight to `SettingsStore::update` → `HoneypotSettingsRow::upsert`
  (`zayden-app/src/config/tables/honeypot.rs:42-63`) with no such seed.
- **Why it matters:** Nothing seeds `guilds` on join —
  `bot/src/handler/guild_create.rs:24-44` does not, and the dashboard's guild
  list comes from the Discord OAuth API, not the DB
  (`dashboard/src/server/guild.rs:69-76`), so a guild with no row is fully
  reachable in the UI. The row first appears when someone chats
  (`bot-modules/levels/src/manager.rs:430`) or opens a ticket
  (`bot-modules/ticket/src/support_guild_manager.rs:93`). So the failing path is
  the *fresh install*: invite the bot, go straight to the dashboard, arm the
  honeypot before anyone has spoken → `23503` surfaced as a generic
  `server_err`, with no indication that chatting once would fix it.
- **Why this is recorded here and not only as a CC:** the underlying hazard is
  **cross-cutting** — *every* `*_settings` table FKs `guilds(id)`
  (`migrations/0003_settings_split.up.sql`, `0015_family_guild_scope.up.sql`,
  `0022_honeypot.up.sql`), so every dashboard `save_*_settings` shares it. What
  makes honeypot the place it is *visible* is that it is the only module with
  **two** writers where exactly one seeds; the asymmetry is the evidence. The
  right fix is almost certainly one owner for the seed rather than a honeypot
  patch — see the follow-up note below.
- **Suggested fix:** Do not add a second ad-hoc `INSERT INTO guilds` to the
  dashboard. Put the seed where it cannot be forgotten: either
  `SettingsStore::update`/`upsert` (one place, covers all nine settings tables
  and both writers) or `guild_create`, which is the natural "we joined this
  guild" event. Then delete the one in `commands/mod.rs:97-102` so there is a
  single owner, per the CC-5 / ticket #2 precedent. **Open this as a
  cross-cutting finding before fixing it here** — a honeypot-scoped fix would
  leave the other eight tables exposed.

### 4. Ban reason is a literal duplicated across two crates  ·  #4  ·  low-med
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
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
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
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
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
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
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/honeypot/tests/policy.rs` (11 tests, all against
  `policy.rs`); untested: `guard.rs` (79 LOC), `message_create.rs` (101 LOC),
  `commands/mod.rs:71-80` (`require_manage_guild`).
- **What:** The existing test file is good and should be said so plainly — it
  pins the exemption matrix, and its comments record *why* each case is the way
  it is ("The user chose 'guild owner only by default'…", `:75-77`), which is
  the opposite of the trivia checklist #6 warns about. It covers the pure
  decision function. It does not cover the code that acts on that decision.
- **Why it matters:** Two of the three untested files hold this audit's two
  `med` findings (#1, #2), which is not a coincidence — the untested surface is
  where they survived. `require_manage_guild` is separately notable: it is an
  **authz gate**, the class [temp-voice #4](temp-voice.md) was ranked `med` for,
  and it is a hand-rolled permission check
  (`interaction.member.permissions.is_some_and(manage_guild)`) sitting *behind*
  a `default_member_permissions(MANAGE_GUILD)` declaration — belt-and-braces
  that nothing verifies still agree.
- **Suggested fix:** Take it in the order the seams allow, not all at once.
  (a) `tests/guard.rs` — pure, no Discord, no DB; the concurrency assertion in
  #2 is the fails-before test for that finding, so it lands *with* #2 rather
  than as separate coverage work. (b) `require_manage_guild` needs a
  `CommandInteraction`, which is the same fabrication problem that ruled
  [verify #1](verify.md) `wontfix` — check whether temp-voice #4's
  `tests/actions_authz.rs` harness (`2503adc1`) generalises before assuming it
  is untestable. (c) `message_create`'s ban/unban sequencing needs an injectable
  HTTP seam that does not exist today; do **not** invent one speculatively —
  fold whatever seam #1's fix requires into #1, and let the coverage follow the
  refactor rather than drive it.

### 8. `GUARD.forget` invalidates a cache the settings change cannot affect  ·  #2  ·  low
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
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
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
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
  one compile-time `query!` (`commands/mod.rs:97`), so no
  [CC-5](_cross-cutting.md#cc-5) runtime-SQL bypass. (That statement seeds the
  `guilds` parent row and is itself finding #3 — the layering is right, the
  ownership of the seed is not.)
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
  `claim` releases it (`:59`, `:67`, `:74`). Findings #1 and #2 are the #3
  exceptions.
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
  boundary question, not a honeypot one, and it predates this crate — recorded
  here only because tracing finding #1 walked through it.
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
