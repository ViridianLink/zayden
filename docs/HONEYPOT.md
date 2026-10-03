# Honeypot Hardening — Counter Analysis & Plan

## Context

`bot-modules/honeypot` ships a single-channel trap: any non-exempt human message in the
configured channel triggers a soft-ban (ban with 24h purge, then unban) and records a
0-point `SoftBan` infraction. The threat model discussed in the prior turn enumerated 16
ways spam bots are likely to evolve around it.

This document works through each of those points, states the concrete counter (or flags it
as uncounterable with a reason), and turns the counterable set into an implementation plan.

Exploration also surfaced **live bugs** in the current module that are independently worth
fixing — they are folded in below and marked **[BUG]**.

---

## Part 1 — Counter matrix

Legend: **✅ Counterable** · **◐ Partial** · **❌ Not counterable**

### A. Reconnaissance

#### A1 — Read channel history before posting ✅
**Counter: seed the honeypot with decoy traffic.**

`bot/src/handler/message_create.rs:25` early-returns on `msg.author.bot()`, and the honeypot
compares author identity nowhere else — so **bot- and webhook-authored decoys are trap-safe
by construction**. Nothing needs to change in the exemption logic to make seeding safe.

Implementation: a cron job on the existing `zayden_core::CronJob` infrastructure
(`bot-modules/zayden-core/src/cron.rs`, registered in `BotState::setup_static_cron` at
`bot/src/state.rs:100-127`) posts a decoy on a jittered schedule and prunes old ones.

Residual risk: a channel where only the bot talks is itself a signature. A webhook with
varied usernames/avatars is materially better camouflage than the bot posting as itself —
see Decision 3.

#### A2 — Metadata-only detection (`last_message_id`, name/topic, zero pins) ✅
**Counter: same seeding as A1, plus set-time camouflage validation.**

Seeding populates `last_message_id`, which is the zero-cost tell available in the
`GUILD_CREATE` payload. On top of that, `/honeypot set` currently does **no validation at
all** (`bot-modules/honeypot/src/commands/mod.rs:89-91`) — add a warning when the chosen
channel's name matches obvious bait patterns (`no-post`, `do-not-post`, `mods-only`, `⚠`,
`🚫`), and strip any suggested default name from the dashboard copy.

#### A3 — Cross-guild fingerprinting ◐
**Partially counterable.**

The correlatable property is "empty-but-postable channel", and seeding removes it — so the
fingerprint collapses to the same fix as A1/A2.

**Uncounterable sub-point:** an attacker can always tell Zayden is present (public member
list) and can always enumerate channels they can see. You cannot hide the bot's presence.
This is harmless on its own — it only becomes a fingerprint when combined with a static
tell, which seeding removes.

#### A4 — Sacrificial probe accounts ❌ (mitigable, not counterable)
**This is the structural one. Flagged as not counterable.**

**Reason:** at the moment of decision, a prober's message and a spammer's message are
identical. Any trap that *acts* emits a signal, and the prober reads the signal. You cannot
build an automated actuator that is invisible to something willing to spend an account
finding out.

What is achievable is degrading the signal's quality, not removing it:
- **Silent/observe mode** (config toggle): record the hit, alert staff, take no Discord
  action and delete nothing. The prober learns nothing; you still get a high-confidence
  label. This is the closest thing to a real counter, and it converts the trap from an
  actuator into a sensor.
- **Delayed + jittered action**: queue the action 30–300s out so the prober cannot bind the
  outcome to a specific channel when probing several.
- **Scored rather than binary action**: outcome varies by account age/roles/history, so a
  single probe returns an ambiguous answer.

Note the current soft-ban makes A4 *maximally* cheap — the probe account is unbanned and
free to reuse the map it just built. Fixing that is B5.

#### A5 — Honeypot maps sold as a service ✅
**Counter: rotation.**

A sold map is only valuable while it is accurate. Support **multiple honeypot channels per
guild** and arm a random subset on a rotating schedule. The admin creates the channels (no
`MANAGE_CHANNELS` needed, no destructive action in the user's guild); the bot only chooses
which are live. Requires widening `honeypot_settings.channel_id` to a set — see Plan §2.

---

### B. Attacking the trap's mechanics

#### B1 — Weaponised false positives (baiting real members in) ✅
**Counter: circuit breaker + tenure-scaled response.**

There is currently no limit — the trap will fire fifty times in a row. Add:
- A per-guild firing-rate breaker (reuse the `moka` pattern already in
  `bot-modules/honeypot/src/guard.rs`): more than N hits in M minutes → stop acting, switch
  to alert-only, ping staff.
- Tenure gate: a member who joined long ago and holds roles gets quarantined + staff-alerted
  rather than banned. Bait then costs an alert, not a member.

#### B2 — Exemption-role farming ✅ (with a trap of its own)
**Counter: validate `exempt_role_id` at the single write path.**

Exploration confirmed `exempt_role_id` is writable from **exactly one place** —
`dashboard/src/server/guild.rs:379-400` (`save_honeypot_settings`), which does no validation.
The slash command doesn't set it at all. One enforcement site, clean.

Two self-assignment vectors must both be checked:
- `reaction_roles.role_id` — queryable (`migrations/0001_v1_init.up.sql:234-242`). Note the
  index is `(guild_id, message_id)`, not `role_id`; fine for a command-path check.
- **`verify` — a hardcoded constant**, `RoleId::new(1_404_640_603_848_839_299)` at
  `bot-modules/verify/src/lib.rs:19`, duplicated at `bot/src/bindings/verify/mod.rs:78`.
  There is **no table backing it**, so a validator that only queries `reaction_roles`
  silently misses it. It is also ungated — anyone who clicks the button gets it.

`levels` is **not** a vector — confirmed zero role-granting code.

#### B3 — Author laundering (webhook / bot / user-installed app) ◐
**Partially counterable.**

- **Webhooks and bots:** currently invisible — `bot/src/handler/message_create.rs:25` drops
  them before the honeypot runs. Counter: move the honeypot check above the bot filter and
  treat a webhook/bot post in the honeypot as an alert (delete the message, alert staff,
  optionally delete the webhook). This must be reconciled with A1 seeding, which
  deliberately posts as a webhook — the seeder's own webhook id must be allowlisted.
- **User-installed apps ◐:** `Message.interaction_metadata` carries the invoking user, so
  attribution is *sometimes* possible. But the app is not in your control and the metadata
  is not guaranteed on every path. Partial at best.

#### B4 — Rate-limit exhaustion of the bot ✅
**Counter: batch, and get off the event loop.**

Confirmed severity: each hit is **2 sequential API calls** (ban + unban), `bulk_ban` appears
**nowhere** in the repo, the `ratelimit` handler is **log-only**
(`bot/src/handler/mod.rs:195-245`), and `dispatch` **awaits handlers inline**
(`bot/src/handler/mod.rs:138`) — so a flood blocks that shard's event processing.

- Coalesce hits in a short window and issue one `bulk_ban` (200 users/call).
- Spawn the action off the event loop so `dispatch` is not blocked.
- The B1 breaker is the backstop when batching still can't keep up.

Note: `bulk_ban` has no unban counterpart, so N unbans remain — which is an argument for
changing the action model entirely (Decision 1).

#### B5 — Soft-ban is not a lockout; repeats are invisible ✅
**Counter: persistent repeat detection + non-zero escalation.**

Three separate defects compound here:
- `HONEYPOT_POINTS: i32 = 0` (`bot/src/bindings/honeypot/mod.rs:15`) — hits never move the
  escalation ladder (`bot/src/bindings/moderation/infraction.rs:98-104`: 1→warn, 2→1h,
  3→8h, 4→28d, ≥5→ban).
- `ACTION_TTL` is 1 minute (`bot-modules/honeypot/src/guard.rs:11`), so the same account
  tomorrow is treated as brand new.
- **[BUG]** `InfractionRow::user_infractions` filters **by `user_id` only, never
  `guild_id`** (`bot/src/bindings/moderation/mod.rs:112,131`) — so simply raising
  `HONEYPOT_POINTS` above zero would leak points across every guild the bot is in. This
  must be resolved deliberately, not by accident. See Decision 4.

Counter: query `infractions` for prior honeypot hits *in this guild* instead of relying on
the moka TTL; on a repeat, ban without the unban.

#### B6 — Audit-log discloses the mechanism ✅
**Counter: neutral audit reason.**

`REASON = "Honeypot: posted in the honeypot channel"` is sent to Discord as the ban reason
(`bot-modules/honeypot/src/message_create.rs:14`, duplicated verbatim at
`bot/src/bindings/honeypot/mod.rs:17`). Anyone with View Audit Log or Ban Members reads the
mechanism. Send a neutral string to Discord; keep the descriptive text on the internal
`infractions` row. Deduplicate the constant while there.

---

### C. Bots that stop looking like bots

#### C1 — Sleeper accounts ✅ (honeypot is already immune)
**Reframed: this defeats age heuristics, not the honeypot.**

The honeypot is one of the few defences a sleeper *doesn't* beat, because it keys on
behaviour-in-a-place rather than account age. No change needed — but this is an argument
against adding an account-age *exemption* to the trap, and for keeping tenure scaling (B1)
as a response modifier rather than a bypass.

#### C2 — LLM-generated per-server contextual spam ◐
**Partially counterable.**

Seeding (A1) is the counter, and it's the same fix. Residual: an LLM reading a seeded
channel may still spot low-quality or repetitive decoys, and that is an arms race on decoy
quality you eventually lose. The repo has an existing `bot-modules/ai` crate, so
LLM-generated decoys are an available escalation — see Decision 3.

#### C3 — Out-of-band payload (bio, status, nickname) ❌ for this module
**Not counterable by the honeypot.**

**Reason:** no channel message is ever posted, so the trap cannot fire by construction.
This needs a different sensor. Note the current wiring: `presence_update` **is** live
(`bot/src/handler/mod.rs:152-155`) so custom status is already observable, but
`guild_member_update` is filtered out (`mod.rs:68`) and the `GUILD_MEMBERS` intent is **not
requested** (`bot/src/main.rs:119-124`) — nickname payloads are invisible today. Separate
feature, out of honeypot scope. See Decision 2.

#### C4 — DM-based delivery ❌
**Not counterable. Hard stop.**

**Reason:** a bot cannot observe DMs between two users — there is no API for it, and the
only technical workaround (a decoy user account reading its own DMs) is a self-bot and a
Discord ToS violation. The sole viable signal is member reports, which is a different
feature (a low-friction `/report` with N-reports-auto-action), not a honeypot change.

#### C5 — Non-message surfaces ◐ — **and one is a live bug**
**[BUG] Threads and forum posts do not trigger the trap.**

`bot-modules/honeypot/src/message_create.rs:41` compares `msg.channel_id.expect_channel()`
against the configured id, so a message in a **thread under the honeypot channel does not
fire**. The dashboard's `TEXT_KINDS` already offers `GuildForum` as a selectable honeypot
channel — and since every forum post is a thread, **forum honeypots are currently
non-functional**. Cheap fix: also match the thread's parent id.

Not counterable by this module: nicknames, scheduled-event names, voice channel status,
stage topics, poll options. All need new events/intents (same blocker as C3).

---

### Additional live bugs found during exploration

| | Location | Issue |
|---|---|---|
| **[BUG]** | `honeypot/src/message_create.rs:78-85` | A failed unban leaves a **permanent ban**; only logged, never retried, and still returns `Some(hit)`. |
| **[BUG]** | `honeypot/src/message_create.rs:54` | `msg.member` absent → `map_or(&[], …)` treats the author as role-less, **silently dropping every role-based exemption**. An admin can be banned by their own trap. |
| **[BUG]** | `dashboard/src/server/modules.rs:120-138` | Disabling the Honeypot module in the dashboard only toggles Discord **command permissions** — the `message_create` hook stays armed. There is no kill switch. |
| **[BUG]** | dashboard write path | `GUARD.forget()` is called by `/honeypot set|disable` but **not** by the dashboard (different process), so exempt-role changes take up to 5 min (`FACTS_TTL`) to apply. `AppEvent::ConfigChanged` already crosses processes via LISTEN/NOTIFY — nothing subscribes `GUARD` to it. |
| | `honeypot/src/commands/mod.rs:71-80` | `require_manage_guild` checks `manage_guild()` only, so an Administrator-without-Manage-Server is rejected — inconsistent with `policy::is_staff` (`policy.rs:47`). |
| | `honeypot/src/guard.rs:58-68` | `claim` is check-then-insert, **not atomic**; two concurrent messages can both win. `moka`'s `get_with` fixes it. |
| | `honeypot/src/guard.rs:32-56` | `facts()` has no single-flight — N simultaneous first-hits = N identical `to_partial_guild` fetches. |
| | everywhere | No `BAN_MEMBERS` precheck; the trap silently errors out if the permission is missing. |

---

## Part 1b — Verified findings that reshape the plan

**F1 — `record_hit` fails on every hit. The honeypot has never recorded an infraction.**
`infractions.user_id` and `moderator_id` are both `NOT NULL REFERENCES users(id)`
(`migrations/0001_v1_init.up.sql:182,186`), and `NewInfraction::record` is a bare `INSERT`
with no `users` upsert (`bot/src/bindings/moderation/mod.rs:58-77`). `users` is written
**only** by `levels` (`manager.rs:320,437`) and `family` (`manager.rs:286,421`). Honeypot
`return Ok(())`s at `bot/src/handler/message_create.rs:35` *before* `levels` runs, and the
bot's own id is never inserted (bot messages are dropped at :25) — so the **moderator FK
alone fails 100% of the time** with SQLSTATE 23503. The ban+unban happens, the error is
logged at `bot/src/handler/mod.rs:186`, and no row is written.
→ Blocks B5's repeat detection entirely. Must be fixed first.

**F2 — the serenity `cache` feature is off workspace-wide** (`Cargo.toml:176`,
`default-features = false`, only `rustls_backend` + compression). So `Message::guild_thread()`
falls through to a live `GET /channels/{id}` — the C5 thread fix needs its own cache or a
forum flood becomes an API flood on the shard event loop.

**F3 — `bulk_ban` exists but is the wrong tool.** `GuildId::bulk_ban`
(`serenity/src/model/guild/guild_id.rs:219`) requires BAN_MEMBERS **and** MANAGE_GUILD — a
stricter set than plain `ban` — and there is **no bulk unban**, so the default `softban`
still needs N unbans. It only helps `action = ban`. Cut it; `tokio::spawn` + a semaphore
solves the real problem (`dispatch` awaiting inline at `bot/src/handler/mod.rs:138`).

**F4 — `/honeypot set` needs zero extra HTTP calls to validate.** The resolved channel
option already carries `kind`, `name`, and `app_permissions` (the bot's computed permissions
*including overwrites*) via `channel.base()`. The whole P0 precheck and the A2 name warning
are pure functions over data already in hand.

**F5 — `SettingsStore::update` is a lossy read-modify-write**
(`zayden-app/src/config/settings_store.rs:74-83`). Adding a background cron writer to the
honeypot row would race the dashboard. This is why per-channel and webhook state go in a
**separate table**, not an array column on the settings row.

---

## Part 2 — Implementation plan

Decisions taken: per-guild configurable action · honeypot module only (no new intents) ·
webhook decoy seeding · fix `user_infractions` to be per-guild, honeypot stays at 0 points.

### PR map

| # | PR | Counters | Depends on |
|---|---|---|---|
| 1 | Infractions: `users` upsert, guild scoping, index | **F1**, B5 | — |
| 2 | Honeypot correctness bugs + extract pure decision core | C5, B6, F2 | — |
| 3 | Command-layer hardening + permission precheck | B1 | 2 |
| 4 | Module toggle disarms; `GUARD` follows config changes | kill switch | 2 |
| 5 | `action` column + tenure scaling + off-loop execution | A4, B1, B4 | 2,3,4 |
| 6 | Circuit breaker + staff alert channel | B1 | 5 |
| 7 | In-guild repeat detection | B5 | 1,5 |
| 8 | Multi-channel rotation | A5 | 5 |
| 9 | Bait-y-name camouflage warning | A2 | 3 |
| 10 | Webhook decoy seeder | A1, A2, C2 | 8 |
| 11 | Exempt-role self-assignment validation | B2 | — |

Hard ordering: **1 before 7** (F1 — the table is empty otherwise). **2 before 5** (the pure
`decide()` extraction is the substrate for the action model). **5 before 6** (the breaker
needs `observe` to fall back to). **8 before 10** (the seeder hangs webhook state on
per-channel rows). PRs 9 and 11 are independent and can ship any time.

---

### PR 1 — Infractions: FK fix, guild scoping, index

`bot/src/bindings/moderation/mod.rs`:
- `NewInfraction::record` (:58) — wrap the INSERT in a `users` upsert CTE, matching the
  `levels` precedent (`INSERT INTO users (id, username) … ON CONFLICT (id) DO NOTHING`).
  Must cover **both** `target_id` and `moderator_id`; dedupe in Rust when they're equal.
- `InfractionRow::user_infractions` (:92) — add a `guild_id: GuildId` param and
  `AND guild_id = $2` to both query arms (:112, :131).
- Call sites: `infraction.rs:83`, `logs.rs:92` (both already have `cx.interaction.guild_id`).

**Behaviour change to call out in the PR body:** `/logs` currently shows a moderator
infractions from *every guild the bot is in* — an information-disclosure bug that is more
urgent than the escalation one. Scoping it also means a user with 4 points in guild A now
resets to 0 in guild B.

Migration `0023_infractions_guild_scope`:
`CREATE INDEX idx_infractions_guild_user_created ON infractions (guild_id, user_id, created_at DESC);`
(no `CONCURRENTLY` — sqlx wraps migrations in a transaction). Serves both PR 7's repeat
lookup and PR 6's breaker. No trigger (`infractions` is not a settings table).

---

### PR 2 — Correctness bugs + pure decision core

**The real deliverable is testability.** New `bot-modules/honeypot/src/decision.rs`:

```rust
pub struct Trigger<'a> {
    author_id: UserId, webhook_id: Option<WebhookId>,
    channel_id: ChannelId, thread_parent: Option<ChannelId>,
    member_roles: RoleSource<'a>, joined_at_unix: Option<i64>, now_unix: i64,
}
pub enum Skip { Disabled, NotArmed, WrongChannel, Decoy, Exempt, Deduped }
pub enum Decision { Skip(Skip), Act(Action) }
pub fn decide(t: &Trigger<'_>, cfg: &TrapConfig, facts: &GuildFacts) -> Decision;
pub fn channel_matches(channel_id, thread_parent, armed: &[ChannelId]) -> bool;
```

`message_create` becomes *gather → `decide()` → `execute()`*. `decide` needs no serenity
HTTP, no DB, no tokio — which is what makes every downstream phase testable.

- **Thread/forum matching (C5).** `src/message_create.rs:41-44` compares the raw channel id.
  Check `msg.channel_type` first; only for thread types resolve the parent via a **new
  `GUARD` cache** `thread_parents: Cache<ThreadId, ChannelId>` (TTL 1h) using `try_get_with`
  for single-flight — per **F2**, without this a 40-post forum burst issues 40 GETs. Cheap
  check first: only resolve if the raw id doesn't already match. Resolution failure → fail
  closed (`WrongChannel`, no action).
- **Failed unban (permanent ban).** Replace `:78-85`. Treat `JsonErrorCode::UnknownBan`
  (10026) as success (a mod beat us to it) — destructuring pattern already in
  `bot/src/bindings/moderation/rules.rs:553-563`. One retry, then set a new
  `HoneypotHit.left_standing_ban: bool`, and have `record_hit` log `InfractionKind::Ban`
  instead of `SoftBan` so the record matches reality. PR 6 alerts on it.
- **Missing `msg.member` (:54).** Model as `RoleSource::{Present(&[RoleId]), Missing}`.
  `Missing` → fetch via a new `GUARD` cache `members: Cache<(GuildId,UserId), Arc<MemberFacts>>`
  (TTL 60s, `try_get_with`); `MemberFacts { roles, joined_at_unix }` also feeds PR 5's
  tenure. `UnknownMember` (10007) → author already gone, skip. Any other error → **fail
  closed**, do not act: acting without knowing roles risks banning staff, exactly what
  `tests/policy.rs` exists to prevent.
- **Neutral audit reason (B6).** Split the duplicated const by audience, exported once:
  `AUDIT_REASON = "Automated moderation action"` (→ Discord) and
  `INFRACTION_REASON = "Honeypot: posted in the honeypot channel"` (→ DB only).
  `bot/src/bindings/honeypot/mod.rs:17` deletes its copy and imports.
- **Atomic `claim`** — `self.recent.entry(key).or_insert(()).await.is_fresh()`.
  **Single-flight `facts()`** — `try_get_with`. Gotcha: it returns `Result<V, Arc<E>>` and
  `serenity::Error` isn't `Clone`, so add a non-user-facing `HoneypotError::Facts(String)`
  variant and include it in the silent arm of `impl Respond` (`src/error.rs:29`).

Tests — new `tests/decision.rs` (pure/sync, house style): thread-under-honeypot fires;
thread-under-other-channel doesn't; webhook message never actioned; missing-roles ≠ no-roles;
`audit_reason` never contains "honeypot" (pins B6 against a future revert).

---

### PR 3 — Command-layer hardening

- `require_manage_guild` (`commands/mod.rs:71-80`) → use `policy::is_staff` (`policy.rs:47`)
  so ADMINISTRATOR counts and there is one definition of "staff" in the crate.
- **Precheck at `/honeypot set`, free per F4** via `channel.base()`: validate `kind` ∈
  `{Text, News, Forum, GuildMedia}` (mirror the dashboard's `TEXT_KINDS`), then
  `app_permissions.view_channel()`, then the action's required permission
  (`ban_members()` for softban/ban; `moderate_members()` + `manage_messages()` for
  `delete_timeout`; nothing for `observe`). Distinct error message per failure.
- Validate **before** touching the DB — `set()` currently defers, writes `guilds`, writes
  settings, *then* replies (`:93-112`).
- Extract `validate_target(kind, app_perms, action) -> Result<Vec<Warning>, SetupError>` as a
  pure fn → new `tests/precheck.rs`.

---

### PR 4 — Module toggle disarms the trap; `GUARD` follows config changes

Migration `0024_honeypot_enabled`: `ALTER TABLE honeypot_settings ADD COLUMN enabled boolean
NOT NULL DEFAULT TRUE;`. A separate column, not `channel_id IS NULL` — *configured* and
*armed* are different concepts, and collapsing them destroys the admin's channel choice on
disable. No trigger work; `honeypot_settings_notify` already fires per-row.

`zayden-app/src/config/tables/honeypot.rs` — the field must be added to **four** places:
struct, `empty()`, the `SELECT` list (:32), and the INSERT/ON CONFLICT/RETURNING lists (:46-59).

`dashboard/src/server/modules.rs::set_module_enabled` — special-case honeypot to also write
`enabled`. Order fails safe toward *trap off*: disabling → DB write first, then the command
overwrite; enabling → overwrite first, DB write last. `ModuleDef::view` (:120-138) should read
`enabled` back so the toggle reflects reality. Note the generalisation (a `settings_flag` hook
on `ModuleDef`) as follow-up rather than building it now.

`GUARD` invalidator — add `spawn_guard_invalidator(rx: broadcast::Receiver<AppEvent>)`
mirroring `SettingsStore::spawn_invalidator` (`settings_store.rs:91-114`) **including** the
`Lagged(n)` → `invalidate_all()` arm; call from `bot/src/main.rs` next to `EventListener::spawn`
with `app.subscribe()`. Closes the gap where a dashboard exempt-role edit took up to 5 min
(`FACTS_TTL`) to reach the bot. Accept the 60s TTL for the `members` cache rather than
enabling moka invalidation closures.

---

### PR 5 — Action model, tenure scaling, off-loop execution

**Column representation: `text NOT NULL DEFAULT 'softban'` + CHECK — not a PG enum.**
The enum is the repo precedent (`infraction_kind`), but if the dashboard ever writes a value
an older deployed bot binary doesn't know, `Row::select` hard-errors, `SettingsStore::get`
propagates, and the `?` at `bot/src/handler/message_create.rs:31` **kills the entire message
pipeline for that guild** — not just the honeypot. Unacceptable blast radius for a config
value slated to grow. Use a hand-written `Decode` that falls back to the default on an unknown
string, with `query_as!` type override `r#"action AS "action!: HoneypotAction""#` (precedent
at `moderation/mod.rs:106`) to keep CLAUDE.md's compile-time-macro rule. Round-trip through
the dashboard's string `ActionForm` is then the identity function.

Migration `0025_honeypot_action` adds: `action text DEFAULT 'softban'`, `veteran_action text`
(nullable — `NULL` means "same as `action`", so tenure scaling is opt-in and current
behaviour is exactly preserved), `tenure_secs integer DEFAULT 604800`, `alert_channel_id
bigint`, `alert_role_id bigint`, `rate_limit_per_hour smallint DEFAULT 10` (0 = off), plus
two CHECK constraints on the action columns. The alert/rate columns land here (one migration,
one sqlx regen) but are consumed in PR 6.

| Action | Discord calls | Permission | Infraction kind |
|---|---|---|---|
| `observe` | none | — | `Warn` |
| `delete_timeout` | `delete_message` + `edit_member(disable_communication_until)` | MANAGE_MESSAGES + MODERATE_MEMBERS | `Mute` |
| `softban` | `ban(24h purge)` + `unban` | BAN_MEMBERS | `SoftBan` |
| `ban` | `ban(24h purge)` | BAN_MEMBERS | `Ban` |

Points stay 0 for all four (decision 4) — the honeypot never drives the escalation ladder.
`observe` is the **A4 counter**: a prober gets no feedback, staff still get the label.

**Off the shard event loop (B4, per F3).** Split into a cheap synchronous part that runs
inline (settings check, channel match, `GUARD.claim` — a miss on >99.99% of messages) and a
`tokio::spawn`ed part (facts/member fetch, Discord action, `record_hit`), bounded by a
process-wide `Semaphore` (~8 permits) so a botnet can't open 200 concurrent requests and
starve serenity's own ratelimiter. **Keep the *decision* inline** so the `return Ok(())`
short-circuit at `handler/message_create.rs:35` stays synchronous; only *execution* moves.
The spawned task logs its own errors.

Dashboard: 3 new DTO fields, `get_guild_settings` mapping, 3 new `save_honeypot_settings`
params, 2 `SelectField`s + 1 number input in the fieldset. **No new component** (`SelectField`
suffices) and **no `registry.rs` change** — no phase adds an 11th store, so the
`db`-without-clone at `registry.rs:46` stays untouched. Share the tenure clamp between
dashboard and slash command via a `parse_tenure_days` helper on the row, per
`MusicSettingsRow::parse_auto_disconnect_secs` (`tables/music.rs:22-28`).

Tests — `tests/action.rs`: parse/as_str round-trip; `default() == SoftBan`; unknown string →
`SoftBan`; and an invariant test asserting the Rust variant list equals the SQL CHECK list
(catches adding a variant without an `ALTER … CHECK`, in the spirit of the existing
`TABLE == "honeypot_settings"` test). `tests/tenure.rs`: `veteran_action = None` → always
`action`; threshold boundary; `joined_at = None` → falls back to `action`, **never** harsher;
clock skew (`joined_at > now`) must not underflow.

---

### PR 6 — Circuit breaker + staff alert

`DashMap`, not moka — you need read-modify-write under one lock, the pattern already
established at `bot-modules/gambling/src/game_cache.rs:9-29`. Keep the clock **out** of the
state machine so it stays pure:

```rust
pub fn record(w: &mut Window, now_unix: i64, cap: u16) -> Verdict;
pub enum Verdict { Allow, Trip, AlreadyTripped }
```

On `Trip`, force the action down to `observe` for a 1h cooldown and alert exactly once
(`Trip` vs `AlreadyTripped` is what makes that possible). This is the **B1** counter: bait
attacks cost an alert, not a mass ban.

Alert delivery has no reusable helper; follow `bot/src/bindings/moderation/rules.rs:539` —
`GenericChannelId::new(as_u64(id)).send_message(…)` with an explicit
`CreateAllowedMentions::new().roles(…)`, **without which the role ping renders but does not
notify**. Reuse the same channel for PR 2's standing-ban alert. Alert failure must never
abort the moderation action.

Tests — `tests/breaker.rs`: below cap always `Allow`; crossing cap yields exactly one `Trip`
then `AlreadyTripped`; cooldown re-arms; window rollover resets; per-guild isolation;
`cap = 0` always `Allow`.

---

### PR 7 — In-guild repeat detection (B5)

Blocked on PR 1 (per **F1** the table is empty for exactly this population). Count prior
honeypot rows in *this guild* over 30 days, matched on `reason = INFRACTION_REASON` and served
by `idx_infractions_guild_user_created`. Matching on `reason` is slightly fragile — the clean
fix is a `source text` column on `infractions`; flag as follow-up and keep the single const as
the source of truth.

Escalation is pure: `escalate(base: Action, prior_hits: i64) -> Action`, with `Action` ordered
`Observe < DeleteTimeout < SoftBan < Ban` so escalation is a monotone `max` and can never
*downgrade* a guild's configured action. **Gate escalation on `base != Observe`** — observe
means "tell me, don't act", and must stay a true dry-run. Run it in PR 5's spawned task, not
inline. Test: `escalate` never returns less than `base`, exhaustive over 4 × 0..4.

---

### PR 8 — Multi-channel rotation (A5)

**Separate `honeypot_channels` table, not a `bigint[]` column** — three reasons: per-channel
state an array can't carry (`armed`, and PR 10's `webhook_id`/`token`/`last_seeded_at`);
**F5**, a rotation cron mutating an array field would race dashboard saves through the lossy
`SettingsStore::update`; and real FK + `UNIQUE (guild_id, channel_id)`.

Migration `0026_honeypot_channels`: the table, a backfill from `honeypot_settings.channel_id`,
and its own `honeypot_channels_notify` trigger (so rotation invalidates cross-process).
`SettingsStore` is strictly one-row-per-guild, so this lives outside it — cache as
`Cache<GuildId, Arc<[ArmedChannel]>>` in `GUARD`, invalidated by PR 4's subscriber.

**Keep `honeypot_settings.channel_id`** as the dashboard-managed *primary*; `honeypot_channels`
is authoritative for *matching* and `channel_id` is a mirror `set()` keeps in sync. New
`/honeypot add` / `/honeypot remove` manage extras. Add a test that `set()` writes both.

Rotation cron: **read `bot/src/cron.rs:83-128` first** — the `TODO(M9-correctness)` there is
real (`pending_jobs` only executes the soonest job or ties), so an eleventh job competes with
the existing ten. Don't assume precise pacing; no sub-minute schedules. jiff-cron takes
**7 fields** (`sec min hour dom mon dow year`).

---

### PR 9 — Bait-y-name camouflage warning (A2)

Tiny and independent. Pure `is_baity(channel_name: &str) -> bool` — lowercase, strip `-`/`_`,
substring-match a small static list (`honeypot`, `spamtrap`, `donotpost`, `nopost`, `modonly`,
…). No `regex` needed. Wired via `channel.base().name` (free per **F4**). **Warn, never block**
— appended to the existing success reply (`commands/mod.rs:117-123`). Test a negative like
`#trapper-keeper` to pin that you didn't naively substring-match `trap`.

---

### PR 10 — Webhook decoy seeder (A1/A2/C2) — ship last, or defer

New ground: no `create_webhook` call exists in the repo (only `Webhook::from_url` in
`webhook_logger.rs:38`). Verified APIs: `ChannelId::create_webhook`
(`serenity/src/model/channel/channel_id.rs:259`), `ExecuteWebhook::username`/`avatar_url`.

Migration `0027_honeypot_decoys` extends `honeypot_channels` with `webhook_id`,
`webhook_token`, `seed_enabled`, `last_seeded_at`. **`webhook_token` is a credential** — never
log it, never return it to the dashboard, never put it in an error message.

**Self-immunity is the critical part, and today it is accidental.**
`bot/src/handler/message_create.rs:25` drops all `author.bot()` messages (webhook messages
carry `bot: true`) *before* the honeypot runs — which is also exactly why the trap cannot
catch bot/webhook spam (**B3**). Add an **explicit** rule in `decision.rs`:
`Skip::Decoy` when `msg.webhook_id.is_some()` (`Message.webhook_id`, `message.rs:104`), so the
seeder stays safe if that early return is ever moved. If you later want "a webhook posting in
the honeypot is suspicious" alerting, compare against the **stored** `webhook_id` — ours →
silent, anyone else's → alert-only, never auto-action (there is no user to ban).

Jitter: jiff-cron is deterministic, so schedule coarsely (hourly) and sleep a random 0–45 min
inside the job, randomising which armed channels get seeded (`rand` is already a workspace
dep). Prune on a ≤14-day horizon (Discord's bulk-delete refuses older messages).
`avatar_url` takes a **URL**, so rotating avatars needs publicly hosted images — an
infrastructure dependency, not just code.

Make identity selection a **seeded pure fn** — `pick_identity(seed: u64, pool: &[Identity])` —
so it's testable without RNG.

**Risk note:** decoys bump the honeypot channel in every member's sidebar, raising the odds a
*human* wanders in. Ship PR 5's tenure scaling and PR 6's breaker before any seeder.

---

### PR 11 — Exempt-role self-assignment validation (B2)

One enforcement site: `dashboard/src/server/guild.rs:379-400` (`save_honeypot_settings`) is
the **only** write path for `exempt_role_id`; the slash command doesn't set it.

Two vectors, and a validator that checks only the first silently misses the second:
- `reaction_roles.role_id` — `SELECT EXISTS(SELECT 1 FROM reaction_roles WHERE guild_id = $1
  AND role_id = $2)`. Index is `(guild_id, message_id)`, no `role_id` index — fine on a
  command path.
- **`verify`** — `RoleId::new(1_404_640_603_848_839_299)` hardcoded at
  `bot-modules/verify/src/lib.rs:19` and duplicated at `bot/src/bindings/verify/mod.rs:78`,
  with **no table** and an ungated button. Known at compile time; check it directly.

`levels` is confirmed **not** a vector (zero role-granting code). Reject the save with a clear
message rather than silently accepting an immunity hole.

---

## Cross-cutting

**Migrations:** `0023_infractions_guild_scope` (PR 1) · `0024_honeypot_enabled` (4) ·
`0025_honeypot_action` (5) · `0026_honeypot_channels` (8) · `0027_honeypot_decoys` (10).
Every `.up.sql` needs a matching `.down.sql`. Only `0026` adds a trigger
(`CREATE OR REPLACE TRIGGER <table>_notify … EXECUTE FUNCTION notify_config_changed()`, per
`0022_honeypot.up.sql:9-12`); `0024`/`0025` inherit `honeypot_settings_notify`.

**SQLx offline cache** — after PRs 1, 4, 5, 7, 8, 10:
```
sqlx migrate run
cargo sqlx prepare --workspace -- --all-features
git add .sqlx
```
`--all-features` is mandatory (CLAUDE.md): `save_honeypot_settings`/`get_guild_settings` are
behind `ssr`, and a plain `--workspace` prepare silently misses them — producing a cache that
passes locally and fails the Docker build.

**Test inventory** — `bot-modules/honeypot/tests/`, all pure/sync, no tokio/DB/HTTP, matching
the house style of the existing `tests/policy.rs` (integration tests in `tests/`, never inline
`#[cfg(test)]`):

| File | PR | Covers |
|---|---|---|
| `policy.rs` (extend) | 2,4 | `is_staff` with ADMINISTRATOR only; `empty().enabled` |
| `decision.rs` (new) | 2,4,7 | channel/thread matching, webhook skip, disabled skip, `escalate` monotonicity |
| `precheck.rs` (new) | 3,5 | channel-kind + per-action permission validation |
| `action.rs` (new) | 5 | round-trip, unknown→default, Rust-enum ↔ SQL-CHECK invariant |
| `tenure.rs` (new) | 5 | boundary, missing `joined_at`, clock skew |
| `breaker.rs` (new) | 6 | window rollover, single `Trip`, cooldown, isolation, `cap=0` |
| `channels.rs` (new) | 8 | set matching, disarmed channels |
| `camouflage.rs` (new) | 9,10 | `is_baity` positives/negatives, seeded identity determinism |

---

## Verification

Per PR, before concluding (CLAUDE.md):
```
cargo +nightly clippy --workspace --all-targets -- -D warnings
cargo test
cargo machete          # PRs 4 and 6 touch Cargo.toml (tokio, dashmap)
```
Iterate with `cargo +nightly clippy -p honeypot`; run the full gate only before merge.
Workspace lints deny `unused_imports`, `rust_2018_idioms`, `elided_lifetimes_in_paths`, and
`future_incompatible`, and CLAUDE.md forbids `#[allow]`/`#[expect]` escapes.

End-to-end, in a test guild:
1. **PR 1 (F1) is the one to verify first and manually** — arm the honeypot, post from a
   throwaway that has never been seen by `levels`/`family`, and confirm a row now appears in
   `infractions`. On `main` today this silently writes nothing.
2. `/honeypot set` against a voice channel → rejected; against a forum channel → accepted.
3. Post in a **thread** under the honeypot channel → fires (regression test for C5; broken on
   `main`).
4. Set `action = observe` → hit produces an infraction row and a staff alert, and **no**
   Discord action (the A4 counter).
5. Revoke the bot's Ban Members, re-run `/honeypot set` → clear error naming the permission.
6. Disable the Honeypot module in the dashboard → post in the channel → nothing happens
   (broken on `main`).
7. Change the exempt role in the dashboard → confirm it applies in the bot within seconds,
   not the 5-minute `FACTS_TTL`.
8. Fire `rate_limit_per_hour + 1` hits from separate accounts → breaker trips once, alert
   fires once, subsequent hits are observe-only.
9. Check the guild audit log → the ban reason must not mention the honeypot (B6).
