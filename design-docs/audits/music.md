# Audit: music

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

One of the healthiest crates. Async concurrency is handled deliberately
(`tokio::sync::Mutex` for the per-guild `GuildPlayer`, `DashMap` for the players
map), settings are read through the M1 `SettingsRegistry` (`cx.app.settings.music`)
rather than ad-hoc, and it has the second-best test coverage in the workspace
(7 integration files: embeds, permissions, player, queue, resolve, spotify,
youtube). No CC-1 (in-memory manager, not DB-generic). Only minor lint debt.

## Findings

### 1. `#[expect]` in embed builder  ·  #7  ·  low
- **Where:** `src/embeds.rs:50`.
- **What / Why / Fix:** One CC-3 escape-hatch. Triage per
  [CC-3](_cross-cutting.md#cc-3); low priority.

### 2. Resolver network calls — confirm timeouts  ·  #3  ·  low
- **Where:** `src/resolve/{youtube,spotify}.rs`, `src/resolve/mod.rs`
  (`#[async_trait]` resolver trait).
- **What:** External HTTP resolution for tracks. Not verified whether every
  outbound call sets a request timeout (checklist #3).
- **Why it matters:** A hung upstream (YouTube/Spotify) without a timeout can
  wedge a resolve future indefinitely.
- **Suggested fix:** Confirm the shared `reqwest::Client` sets a
  `.timeout(...)`; if not, add one. Quick check, likely already fine.

### 3. `settings` (default-volume) belongs on the dashboard  ·  #8  ·  low
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Scope decision (2026-07-25):** *Partial* move, not the wholesale migration the
  finding proposed. The owner's call: music settings are tweaked **live** as
  listeners and songs change, so the playback-behaviour fields stay in Discord;
  only the **admin setup** fields move to the web.
  - **→ Dashboard:** `dj_role_id`, `auto_disconnect_secs`, `announce_now_playing`.
  - **→ Stay editable in-bot:** `default_volume`, `stay_connected`, `autoplay`.
  - Moved fields lose their `/music settings` command options (dashboard becomes the
    single editor) but remain in the command's read-only embed, mirroring the
    loadout `refresh` precedent in [CC-8](_cross-cutting.md#cc-8).
- **Fix (2026-07-25):** Split `music_settings` between the two editors so that
  **every column has exactly one writer** — the divergence risk CC-8 flags, avoided
  without moving the live-tweak fields off Discord.
  - **Dashboard (new):** `save_music_settings` server fn in
    `dashboard/src/server/guild.rs`, gated by the existing `admin_app` /
    `guild_admin_context` authz like every other `save_*_settings`; `get_guild_settings`
    now also reads the music row into three new `GuildSettings` DTO fields. A "Music"
    `fieldset` on the guild-settings page renders DJ Role (`RoleSelect`),
    Auto-disconnect (`SettingField`) and Announce Now Playing (new `ToggleField`),
    with a note pointing volume/24-7/autoplay back to Discord.
  - **`ToggleField`** (`ui/components/settings.rs`) renders a bool as a two-option
    `<select>` rather than a checkbox: an unchecked checkbox submits *nothing*, which
    would make "off" and "field absent" indistinguishable to the server fn, so the
    off state would never round-trip.
  - **Bot:** the four moved sub-options (`dj_role`, `clear_dj_role`,
    `auto_disconnect_secs`, `announce_now_playing`) are gone from `Command::register`
    and from `settings::run`'s option parsing. All six fields remain in the view
    embed, which gained a footer naming the dashboard as the editor of the moved ones.
  - **Validation moved with the field.** `auto_disconnect_secs` went from a Discord
    `Integer` option (bounded by Discord itself) to a dashboard free-text field, so
    `MusicSettingsRow::parse_auto_disconnect_secs` now owns the normalisation:
    unparseable → `DEFAULT_AUTO_DISCONNECT_SECS` (120), otherwise clamped into
    `0..=MAX_AUTO_DISCONNECT_SECS` (600). This matters beyond tidiness — a negative
    value would pass through `as_u64` in `commands/ctx.rs:71` and become a huge `u64`,
    silently disabling auto-disconnect. `empty()` now seeds from the same constant.
  - **Verification:** `bot-modules/music/tests/settings_surface.rs` (3 tests) pins the
    command surface by serialising `Command::register()` and asserting the `settings`
    subcommand exposes *exactly* `default_volume`/`stay_connected`/`autoplay`.
    **Fails-before / passes-after confirmed:** against `HEAD`'s command definition, 2
    of the 3 fail (`found options: ["dj_role", "clear_dj_role", "default_volume",
    "auto_disconnect_secs", "announce_now_playing", "stay_connected", "autoplay"]`);
    all 3 pass after. 4 more in `zayden-app/tests/config_settings.rs` cover the
    clamp/fallback. The dashboard's own server fn is not directly covered — it needs a
    live `PgPool` + an OAuth session, and `dashboard` compiles its SSR code behind a
    feature flag that plain `cargo test` doesn't enable (see
    [CC-6](_cross-cutting.md#cc-6)); the pure logic it delegates to *is* covered.
  - **Gate:** `cargo +nightly clippy --workspace --all-targets -D warnings` clean,
    `cargo test` green, `-p dashboard --features ssr` and
    `--target wasm32-unknown-unknown --features hydrate` both clean. No new
    `#[allow]`/`#[expect]`. No `query!`/`query_as!` added or changed, so no `.sqlx`
    delta; no `Cargo.toml` dep change, so no `cargo machete` run.
  - **Residual / follow-ups:**
    1. **`announce_now_playing` is a soft stub.** It is stored, edited and displayed,
       but grep finds **no consumer** in the playback path — nothing reads it before
       posting a now-playing message. Checklist #2; worth its own finding (wire it up
       in `player`/`events`, or drop the column). Pre-existing, not introduced here.
    2. The settings page's `Resource` keys on `guild_id` + the create-creator action
       only, so a **clamped** `auto_disconnect_secs` isn't reflected until reload. This
       is the pre-existing behaviour of *every* `save_*` form on the page, left alone
       rather than changed under this finding.
    3. CC-8's "config stranded in-bot" list still holds `ticket`, `suggestions` and
       `reaction-roles`.
- **Where:** `src/commands/settings.rs` (writes `MusicSettingsRow`,
  `default_volume`).
- **What:** A one-shot config command with no dashboard equivalent yet, though it
  is the same shape as the settings already on the web.
- **Why it matters:** Config is the dashboard's domain; the `MusicSettingsRow`
  store already exists, so a web field is cheap.
- **Suggested fix:** Add a music section to the guild-settings page
  (`save_music_settings` server fn); keep playback + control panel in-bot. See
  [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: clear `commands/` · `components/` · `resolve/` · manager /
  player / queue / voice split; settings via `SettingsRegistry`.
- #1 DB access: n/a — playback state is in-memory by design; no ad-hoc SQL.
- #3 Async: **correct** — `tokio::sync::Mutex` held across `.await` is
  intentional and safe; `DashMap` entries not held across `.await`.
- #4 Stringly typing: control-panel routing is namespaced; no raw domain strings.
- #6 Tests: 7 integration files covering real behaviour (queue ops, permission
  gating, resolver parsing).

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lens: state/cache correctness (multi-guild aliasing)._

### DS-1. `VoiceOccupancy` keyed by `UserId` only → premature auto-disconnect when a listener is in two guilds' voice at once  ·  Pass 6+2  ·  low-med
- **Status:** `complete — 19ff2826`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-22):** Re-keyed the map to `DashMap<(UserId, GuildId), ChannelId>`
  (`occupancy.rs`) so a user's presence is tracked independently per guild, and
  scoped `update`'s `None`-branch removal to `remove(&(user_id, guild_id))` — a
  disconnect in one guild no longer evicts the user from another. All consumers
  (`non_bot_count`, `channel_of`, `members_in_channel`, `guild_create`) already
  pass `guild_id`, so the public API is unchanged and no call sites moved;
  `channel_of` is now a direct keyed `get`. Regression test added at
  `bot-modules/music/tests/occupancy.rs` (fails-before / passes-after): it drives
  `update` with deserialized `VoiceState`s to prove joining/leaving guild B leaves
  guild A's count intact. **Residual (documented, not fixed):** the secondary
  `guild_delete` sweep the finding suggests (evict stale members when the bot is
  removed from a guild) is left as a follow-up — it is a separate leak, not part of
  the cross-guild aliasing this task closed.
- **Where:** `bot-modules/music/src/occupancy.rs:8` (`members: DashMap<UserId, (GuildId, ChannelId)>`),
  `:25-38` (`update`); consumed by `non_bot_count` (`:41-54`) which the
  auto-disconnect path reads (`bot-modules/music/src/events.rs:111-121`). Wired at
  `bot/src/handler/voice_state_update.rs:17` (one `VoiceState` per event).
- **What:** Discord lets a single user be connected to voice in **multiple guilds
  simultaneously**. The occupancy map keys by `UserId` alone and stores a single
  `(GuildId, ChannelId)`, so a user's presence in guild B **overwrites** their
  presence in guild A. Worse, when that user later disconnects from B, `update`
  sees `channel_id = None` and `remove`s the user **globally**, dropping them from
  guild A too — even though they never left A.
- **Failure scenario:** The bot plays music in guild A's voice channel with exactly
  one human listener X. X also joins voice in guild B (same bot present). The
  guild-B `VoiceStateUpdate` runs `insert(X, (B, chanB))`, overwriting `(A, chanA)`.
  Now `non_bot_count(A, chanA, bot)` returns 0 → the idle timer starts →
  `auto_disconnect_secs` later the bot leaves guild A's channel and drops the queue,
  despite X still sitting in it. If X leaves B before the timer, the `None` update
  removes X entirely, so A stays at 0 until X emits another voice event in A.
- **Why it matters:** User-visible: music stops for a present listener. Rare
  (needs a shared user across two guilds both running the bot) but fully
  deterministic when it occurs, and the `None`-removes-globally half also corrupts
  the count on plain channel switches between guilds.
- **Confidence:** confirmed (map is keyed by `UserId`; `update`/`remove` traced).
- **Suggested fix:** Key the map by `(UserId, GuildId)` (or store a per-user set of
  `(GuildId, ChannelId)`), and scope `update`'s `None` removal to the event's
  `guild_id` rather than the whole user. Also add a `guild_delete` sweep to evict
  stale members when the bot is removed from a guild (secondary leak).

### DS-2. Concurrent first-play double-start: two `/play`-family calls when idle both `play_input` → overlapping audio + orphaned uncontrollable handle + double queue-advance  ·  Pass 2 (double-submit)  ·  med
- **Status:** `complete — 1e3a42c6`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/music/src/commands/play.rs:68-115` (`enqueue`) — the
  `should_start = guard.current.is_none()` check (`:77`) and the queue lock are
  **released** (`:85`) before the track is popped (`:91-94`) and
  `voice::start_playback` runs (`:96-105`); `start_playback`
  (`bot-modules/music/src/voice.rs:97-139`) only sets `guard.current` under the
  guard `if guard.generation == generation` (`:132`), which does **not** dedupe two
  starts issued at the *same* generation. No per-guild interaction serialization
  exists (`commands/mod.rs:363-399` dispatches each subcommand directly; each
  interaction runs on its own tokio task).
- **What:** The gambling module gates repeat game submits with the atomic
  `GameCache::check_and_set` (5 s). The music start-playback transition has **no**
  equivalent idempotency guard. When nothing is playing (`current.is_none()`), two
  interactions can both observe `should_start == true`, both push, both pop, and
  both call `start_playback` with the identical pre-increment `generation` (read at
  `play.rs:84`). `advance()` — the only thing that bumps `generation` — is never
  called on the enqueue path, so both starts pass the `generation == generation`
  set-current guard: the second overwrites `guard.current`, orphaning the first
  `TrackHandle` (it keeps playing but is no longer referenced, so skip/stop/volume
  can never touch it). Both `songbird` inputs play simultaneously → overlapping
  audio. Both End events carry the same (still-un-bumped) `generation`, so when the
  orphan track ends its `TrackEndNotifier` passes the `guard.generation != self.generation`
  check and advances the queue a second time, dropping a queued track.
- **Failure scenario:** Bot is connected but idle (queue empty, `current = None`).
  A user double-invokes `/play <a>` then `/play <b>` (or `/play` + `/playtop`) within
  the resolve+stream network window (hundreds of ms — `resolve_head` and
  `resolver.stream` are both network round-trips, so the window between the
  `should_start` check and `current` being set is wide). Both pass `should_start`,
  both `play_input`: tracks A and B play over each other. `guard.current` = B; A's
  handle is orphaned. `/skip` stops B and advances; A keeps playing until its
  natural end, at which point A's `TrackEndNotifier` (generation still matching)
  fires and advances the queue again, silently skipping the track that should have
  played next.
- **Why it matters:** User-visible garbled playback and a track that no control
  command can stop, plus a queue position silently lost. Trivial for a user to
  trigger by double-clicking / spamming `/play` while idle; also reachable by two
  users issuing play-family commands at once. Same double-submit class the audit
  found gambling had *closed* with `GameCache` — music simply lacks the guard.
- **Confidence:** confirmed (lock released between `should_start` and
  `start_playback` traced; `generation` never bumped on the enqueue path, so the
  set-current guard cannot distinguish two same-generation starts; no per-guild
  serialization).
- **Suggested fix:** Make the "start if idle" decision and the transition to a
  non-idle state atomic. Simplest: under the single `enqueue` lock, set a
  `starting`/`current`-reservation flag (or bump `generation` and stash the intended
  track) so a concurrent caller sees `should_start == false` and only enqueues.
  Alternatively gate the whole play path behind a per-guild
  `check_and_set`-style guard mirroring gambling's `GameCache`.

### DS-3. `announce_now_playing` is stored, edited and displayed but never read → track announcements are an inert setting  ·  Pass 1 (silent failure) / #2  ·  med
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `zayden-app/src/config/tables/music.rs:13` (column),
  `bot-modules/music/src/commands/settings.rs` (embed field) and
  `dashboard/src/server/guild.rs` (`save_music_settings`) — every *writer* and
  *reader-for-display*. There is **no** consumer: a workspace grep for
  `announce_now_playing` returns only the row definition, the settings editors and
  the view embed. `bot-modules/music/src/events.rs:47-76` (`TrackEndNotifier`) — the
  natural consumer — advances the queue and starts the next track without posting
  anything.
- **What:** The guild can toggle "Announce Now Playing" in Discord *and* (since
  [#3](#3-settings-default-volume-belongs-on-the-dashboard--8--low)) on the
  dashboard, but the flag changes nothing. Same class as
  [family DS-1](family.md) (`/block` never enforced): a setting with a UI, a column
  and no enforcement.
- **Failure scenario:** An admin enables "Announce Now Playing" expecting a message
  per track. Nothing is ever posted, at any setting value. The toggle is
  indistinguishable from a no-op, and the dashboard now advertises it as a
  supported feature.
- **Confidence:** confirmed (grep: no consumer).
- **Intended behaviour (owner, 2026-07-25)** — this is a *spec*, not just a
  wire-up:
  1. **Default (no announce channel configured):** announcements go to the channel
     the command was invoked in, as a reply/followup. The plumbing already exists —
     `GuildPlayer::text_channel` is captured from the interaction at
     `commands/ctx.rs:69` and is already used this way for the background
     playlist-truncation notice (`commands/play.rs:148-158`).
  2. **When an admin sets an announce channel:** track announcements go *there*
     instead — e.g. `"Song finished, now playing …"`.
  3. **Direct command feedback is unaffected** and stays on the interaction
     response. Only the unprompted, event-driven announcements are routed; a user
     who runs a command still gets their answer where they ran it.
- **Suggested fix:** Add a nullable `announce_channel_id` to `music_settings`
  (**needs a new migration**, `0018_*`, plus a `.sqlx` regen against an empty
  freshly-migrated DB per [CC-5's residual](_cross-cutting.md#cc-5)), surface it as
  a `ChannelSelect` in the dashboard's Music panel next to the existing toggle
  (admin setup → web, per #3's split), and consume both in `TrackEndNotifier`:
  when `announce_now_playing`, post to `announce_channel_id` if set, else
  `GuildPlayer::text_channel`. Decide explicitly whether an unset toggle silences
  announcements entirely or only suppresses the per-track message.
