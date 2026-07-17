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
