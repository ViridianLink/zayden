# Audit: cross-cutting (workspace-wide)

_Audited: 2026-07-17 · Commit: `2833ce8`_

These are findings that recur across many crates. Recording them once here keeps
the per-module files short — a module file cites the relevant `CC-#` and only
adds its module-specific detail (e.g. exact `path:line`).

## Summary

The workspace is in good overall health: compile-time `query!`/`query_as!` is the
norm, `unwrap()`/`expect()` on live paths is rare, and the newer crates
(`destiny2`, `ticket`, `marathon`, `palworld`) follow the concrete-`PgPool`
convention. The dominant residual issue is an **architectural split**: roughly
half the modules still carry the DB-generic `async_trait` manager pattern that
the implementation spec deliberately removed from `ticket` and never used in
`destiny2`. Secondary themes: inline `#[cfg(test)]` modules violating the
`tests/`-only convention, a cluster of `#[expect(...)]` lint escape-hatches, and
three genuine runtime-SQL bypasses. A newer, forward-looking theme (**CC-8**):
now that the web dashboard is live and already owns much of the settings surface,
a swath of in-bot config/`setup` commands and data-dense displays would be better
served by the website — and two `setup` commands already duplicate its writes.

## Findings

### CC-1. DB-generic `async_trait` manager pattern (should be concrete `PgPool`)  ·  #1  ·  high

- **Where:** manager traits declared `<Db: Database>` / `Pool<Db>` and only ever
  implemented for `Postgres`. Present in: `gambling` (pervasive — `models/*`,
  `commands/*`, `games/*`), `family` (`family_manager.rs` + all commands),
  `lfg` (`guild_manager.rs`, `models/*`, all commands/components),
  `temp-voice` (`voice_channel_manager.rs`, `guild_manager.rs`, actions,
  components), `levels` (`sqlx_lib.rs`), `reaction-roles`
  (`reaction_roles_manager.rs`), `suggestions` (`guild_manager.rs`),
  `gold-star` (`manager.rs`), and the `zayden-core` traits that generalise them.
- **What:** The manager traits are generic over the sqlx `Database` and take
  `Pool<Db>`, forcing `#[async_trait]` (heap-boxed futures) and splitting each
  trait's SQL into a separate `impl … for XxxTable` in `bot/src/bindings/*`. The
  DB is always Postgres — there is exactly one impl per trait.
- **Why it matters:** This is the precise indirection the spec removed from
  `ticket` in Milestone 1 ("removed both generic traits and moved the DB/sqlx
  code concrete into the ticket module") and never introduced in `destiny2`. The
  workspace is now split between two conventions for the same job. The generic
  form costs an allocation per call (`async_trait`), scatters a module's SQL away
  from the module, and buys nothing — there is no second database.
- **Suggested fix:** Migrate the remaining generic managers to concrete `PgPool`
  inherent methods (or non-generic traits) with the `query!`/`query_as!` bodies
  living in the module crate, mirroring `ticket::TicketRow` /
  `destiny2::db`. Drop `async_trait` as each one is converted (native
  `async fn` in traits is stable). Do it one module per PR — `gold-star` and
  `levels` are the smallest starting points; `gambling` is the largest.

### CC-2. Inline `#[cfg(test)] mod tests` in `src/` (convention violation)  ·  #6  ·  med

- **Where:** `bot/src/registry/dispatch_map.rs:103`,
  `bot-modules/palworld/src/commands/breed_plan.rs:147`,
  `bot-modules/gambling/src/components/tictactoe.rs:509`,
  `bot-modules/family/src/family_manager.rs:125`,
  `bot-modules/family/src/commands/information/siblings.rs:90`,
  `bot-modules/zayden-core/src/snowflake.rs:13`,
  `bot-modules/temp-voice/src/voice_channel_manager.rs:168`,
  `bot-modules/temp-voice/src/commands/mod.rs:610`,
  `zayden-app/src/entitlement/types.rs:144`.
- **What:** Nine `#[cfg(test)] mod tests` blocks live inline in `src/`.
- **Why it matters:** The project convention (and the audit checklist #6) is that
  tests live in `tests/` integration files, never inline in `src/`. These
  predate or bypass that rule.
- **Suggested fix:** Move each to the crate's `tests/` directory. Where the test
  reaches private items, expose the minimum surface (or a `pub(crate)` test
  helper) rather than keeping the test inline. `bot` has no lib target, so
  `dispatch_map.rs`'s test needs either a lib target or relocation of the tested
  logic into a lib crate.

### CC-3. `#[expect(...)]` lint escape-hatches  ·  #7 / #2  ·  low–med

- **Where (22 sites):** `bot/src/handler/mod.rs:121`,
  `bot/src/bindings/gambling/{goals,daily,dig,work}.rs`,
  `bot/src/bindings/lfg/mod.rs:159,189`, `bot/src/bindings/levels/mod.rs:105`,
  `bot/src/bindings/temp_voice/mod.rs:132`,
  (~~`bot/src/bindings/moderation/infraction.rs:210`~~ `#[allow(clippy::too_many_arguments)]`
  **removed** in the bot DS-2 revival — the args were bundled into a `Case`
  struct; this was also uncompiled dead code at the time it was inventoried),
  `bot-modules/music/src/embeds.rs:50`, `bot-modules/gambling/src/utils.rs:85`,
  `bot-modules/gambling/src/models/mod.rs:74`,
  `bot-modules/gambling/src/commands/tictactoe.rs:136,151,175,182`,
  `bot-modules/gambling/src/commands/gift.rs:37`,
  `bot-modules/gambling/src/common/shop/items.rs:47,192`,
  `bot-modules/gambling/src/games/lotto.rs:118`,
  `bot-modules/family/src/commands/tree.rs:71`,
  `bot-modules/lfg/src/actions/leave.rs:19`,
  `bot-modules/lfg/src/cron/reminders.rs:20`,
  `bot-modules/destiny2/src/raid_guides/mod.rs` (×6),
  `bot-modules/destiny2/src/endgame_analysis/sheet/tier.rs:104`,
  `bot-modules/destiny2/src/loadouts/record.rs:89`,
  `zayden-app/src/entitlement/service.rs:78`,
  `dashboard/src/web/routes_login.rs:94`.
- **What:** `CLAUDE.md` says do not use `#[allow]`/`#[expect]` to silence clippy
  "unless absolutely necessary." Some are justified (documented `reason =` for a
  genuine invariant), but several silence `too_many_arguments`,
  `cast_sign_loss`, `future_not_send`-on-dead-code, or `dead_code` — smells that
  usually point at a refactor rather than a suppression.
- **Why it matters:** Each escape-hatch is a small deferred cleanup; in
  aggregate they erode the "-D warnings, no allow" guarantee the gate is meant
  to provide.
- **Suggested fix:** Triage per site. `too_many_arguments` → bundle args into a
  struct. `cast_sign_loss` → use the checked/`try_into` path or a domain type
  that is unsigned by construction. `dead_code`/`future_not_send`-on-stub (see
  CC-4) → delete the stub. Keep only the ones documenting a true compile-time
  invariant (e.g. the `const fn` builder panics in `raid_guides`, which are the
  better fix target in CC-5).

### CC-4. `tictactoe` dead `GameState` stub  ·  #2  ·  low

- **Where:** `bot-modules/gambling/src/commands/tictactoe.rs:175,182`
  (`#[expect(clippy::future_not_send, reason = "dead code within GameState stub")]`)
  and the `#[expect(dead_code, reason = "reserved for future implementation")]`
  at `bot-modules/gambling/src/common/shop/items.rs:192`.
- **What:** Self-described dead/stub code retained behind `#[expect]`.
- **Why it matters:** Checklist #2 — soft stubs that compile but do nothing. They
  carry maintenance weight and their `#[expect]`s inflate CC-3.
- **Suggested fix:** Delete the dead stub, or wire it up. The mandate ("optimize
  for the correct end state, not the low-churn path") favours deletion until the
  feature is actually built.

### CC-5. Runtime `sqlx::query(...)` bypassing compile-time macros  ·  #1  ·  med

- **Where:** `bot/src/bindings/gold_star.rs:83` (and the `SELECT` above it),
  `zayden-app/src/entitlement/service.rs:111,309`,
  `dashboard/src/middleware/auth.rs:35`.
- **What:** Hand-written `sqlx::query("…").bind(…)` instead of `query!` /
  `query_as!`.
- **Why it matters:** `CLAUDE.md` mandates compile-time macros so SQL is checked
  against the schema at build. These three sites lose that guarantee (and the
  `.sqlx/` offline cache coverage). Note these are the *only* genuine runtime-SQL
  sites — the CC-1 generic-trait modules still use macros in their concrete
  impls, so they are not part of this finding.
- **Suggested fix:** Convert to `query!`/`query_as!` and regenerate `.sqlx/`.
  `gold-star` also has CC-1, so fold this into its concrete-`PgPool` migration.

### CC-6. Test-coverage gaps  ·  #6  ·  med

- **Where:** crates with **zero** `tests/` files: `ticket`, `lfg`, `family`
  (has only inline — see CC-2), `levels`, `reaction-roles`, `suggestions`,
  `gold-star`, `llamad2`, `verify`, plus `bot` and `dashboard` (no lib target,
  so integration tests are structurally awkward — noted, not blamed).
- **What:** Several sizeable crates ship no integration tests (`lfg` ≈3.3k LOC,
  `ticket` ≈1.3k LOC, `family` ≈1.5k LOC). Well-covered counter-examples:
  `marathon` (13), `palworld` (12), `music` (7), `destiny2` (3).
- **Why it matters:** These crates carry real branching logic (LFG post
  lifecycle, ticket open/close, family relationship graph) with no regression
  net.
- **Suggested fix:** Add `tests/` integration coverage for the pure logic first
  (relationship resolution, LFG slot/alt bookkeeping, ticket state transitions).
  DB-touching paths can follow once a test-pool harness exists.

### CC-7. Component `custom_id` string routing (deferred stringly-typing)  ·  #4  ·  low

- **Where:** `bot/src/bindings/gambling/{prestige,blackjack}.rs`,
  `bot-modules/gambling/src/components/{tictactoe,higherlower}.rs`,
  `bot-modules/levels/src/components/levels.rs:36`.
- **What:** Interaction routing on `custom_id.as_str()` string matches. The M2
  milestone explicitly logged the "component-`custom_id` enum for
  gambling/levels" as an optional deferral.
- **Why it matters:** Guessable string ids scattered across match arms; a typo
  compiles. Lower priority than CC-1 because these are local routing switches,
  not domain data.
- **Suggested fix:** Introduce a per-component `CustomId` enum with
  `as_str`/`FromStr`, following the temp-voice/LFG namespaced-id approach.

### CC-8. Features better served by the (now-live) web dashboard  ·  #8  ·  med

- **Where:** config/`setup` commands and data-dense displays across `lfg`,
  `temp-voice`, `music`, `ticket`, `suggestions`, `reaction-roles`, `gambling`,
  `levels`, `destiny2`, `palworld`.
- **What:** The dashboard (`dashboard/`) is live and already owns a growing slice
  of what used to be bot-only. Its current mutation surface
  (`dashboard/src/server/`) is: `save_support_settings`, `save_channel_settings`,
  `save_role_settings`, `save_temp_voice_settings`, `save_lfg_settings`,
  `set_module_enabled`, and tier/upgrade (Ko-fi). Destiny2 loadout CRUD was
  **already** moved to the website (TODO M3 3c). Two consequences fall out:
  1. **Active duplication.** `bot-modules/lfg/src/commands/setup.rs` and
     `bot-modules/temp-voice/src/commands/setup.rs` write the *same*
     `lfg_settings` / `temp_voice_settings` rows the dashboard now writes. Same
     for support/channels/roles config commands. Two editors, one table.
  2. **Config still stranded in-bot.** `music` (`commands/settings.rs`,
     default-volume), `ticket` (support-guild config), `suggestions` config, and
     `reaction-roles` (`add`/`remove` mapping CRUD) have **no** dashboard
     equivalent yet, though they are the same shape as things already moved.
- **Why it matters:** With the website live, the bot's config/admin/CRUD surface
  and its data-dense read views are the weakest fit for Discord: `setup` commands
  are one-shot forms better as a web page; leaderboards/profiles/tier-lists are
  data-dense views a Discord embed renders poorly (paged buttons, field limits);
  and every duplicated write path is a divergence risk. The destiny2 loadout move
  already set the direction — this finding extends it workspace-wide.
- **The heuristic (dashboard vs. bot):**
  - **→ Dashboard:** one-shot config/`setup`, admin CRUD of reference data,
    rich/paged read-only displays.
  - **→ Stay in bot:** anything needing live Discord context — gameplay
    interactions, message/voice/reaction events, moderation actions, per-message
    component flows (join/leave/kick/claim/playback).
- **Candidates by module (direction, not defects):**
  - `lfg` — `setup` (**duplicates** `save_lfg_settings`); `tags` management (CRUD)
    → dashboard. Keep create/join/leave/kick (live post interaction) in-bot.
  - `temp-voice` — `setup` (**duplicates** `save_temp_voice_settings`) → dashboard.
    Keep `panel` + live channel mutations (claim/kick/transfer/limit) in-bot.
  - `music` — `settings` (default-volume) → dashboard (no server fn yet). Keep
    playback + control panel in-bot.
  - `ticket` — support-guild / panel config → dashboard. Keep open/close/claim
    interactions in-bot.
  - `suggestions` — channel/threshold config → dashboard. Keep the submit modal +
    vote reactions in-bot.
  - `reaction-roles` — `add`/`remove` mapping CRUD → dashboard (browsable list of
    message→emoji→role maps). Keep the reaction event handler in-bot.
  - `gambling` — `leaderboard`, `profile`/stats → dashboard read views. Keep all
    games/economy actions in-bot.
  - `levels` — `leaderboard`, `rank` → dashboard read views. Keep the message-XP
    accrual in-bot.
  - `destiny2` — tier-list + loadout **browsing** → dashboard read views (loadout
    *editing* already moved). Keep autocomplete/`refresh` in-bot.
  - `palworld` — breed-plan / Paldex **display** → dashboard read views. Keep
    save-upload + live server ops in-bot.
- **Suggested approach:** For each duplicated config command, make the dashboard
  the single editor and either remove the bot command or reduce it to a
  deep-link/read-only echo (mirror the loadout `refresh` pattern: the bot reloads
  cache, the website edits). Build the missing config pages (music/ticket/
  suggestions/reaction-roles) against the existing `SettingsRegistry`. Treat the
  read-view migrations as UX upgrades, lower priority than de-duplicating writes.

## Deep-sweep findings

_Deep sweep pass over the whole workspace, 2026-07-17. These are latent
defects that sit **underneath** CC-1…CC-8 — the concrete failure scenarios the
first-pass structural findings only hinted at. Per-module detail lives in the
`DS-#` entries of each module file; this section records the one genuinely
cross-cutting theme plus an index._

### CC-9. Read-modify-write on economy/counter rows with **absolute** overwrite (race class)  ·  #3  ·  high

- **Where (pattern):** the command-layer `save`/`save_*` methods that persist a
  whole in-memory row with `INSERT … ON CONFLICT DO UPDATE SET col =
  EXCLUDED.col` (absolute), while sibling mutations on the *same or related*
  rows use atomic `col = table.col + $n`. Confirmed instances:
  `gambling` `/send`, `/gift`, `confirm_prestige` (see
  [gambling.md DS-1…DS-4](gambling.md)). The generic-manager split (CC-1) is what
  makes this easy to miss — the racy read happens in the module crate, the write
  semantics live in `bot/src/bindings/*`, so no single file shows the hazard.
- **What:** `row = Handler::row().await` → mutate in memory → `Handler::save(row)`
  with an absolute upsert. Two interactions in the same tick (Discord dispatches
  each interaction on its own tokio task — see
  `bot/src/handler/interaction/mod.rs:168`) both read the pre-image and the
  second `save` clobbers the first. Where the *counterpart* write is an atomic
  increment (crediting another user, a shared pool, an inventory), the increment
  stacks while the guard is lost → duplication / limit-bypass. Where both sides
  are the *same* absolute row, the lost update instead silently drops one action
  (data loss, not duplication).
- **Why it matters:** double-click / macro spam is trivial for a user to trigger;
  the payoff is minted currency or bypassed daily caps. This is the highest-value
  defect class found in the sweep.
- **Suggested fix (uniform):** move each check-then-act into a single transaction
  with a conditional/atomic write — `UPDATE … SET col = col ± $n WHERE <guard>`
  and assert `rows_affected == 1` — instead of read → mutate → absolute save. This
  also removes the debit/credit-in-separate-transactions atomicity gap. Prefer
  fixing this at the same time as the CC-1 concrete-`PgPool` migration, since both
  touch the same `save` methods.
- **Wager games traced (mostly clean):** `blackjack`/`higherlower` gate repeat
  plays with the atomic `GameCache::check_and_set` (5s) and debit via the **atomic**
  `bet` decrement, so intra-game double-submit does *not* duplicate. The one
  residual is that `bet.sql` lacks a `WHERE coins >= bet` floor, allowing an
  overdraft when a *different* command mutates the balance between the app-layer
  check and the decrement (see [gambling.md DS-5](gambling.md)).

### Deep-sweep index

| ID | Module | Class | Severity | Confidence |
|----|--------|-------|----------|------------|
| DS-1 | gambling | non-atomic + racy `/send` transfer (mint) | high | confirmed |
| DS-2 | gambling | `/gift` daily-cap double-submit (mint) | high | confirmed |
| DS-3 | gambling | prestige→lotto `ON CONFLICT` `2×`/wipe | med | confirmed |
| DS-4 | gambling | `confirm_prestige` no button idempotency | med | confirmed |
| DS-1 | lfg | fireteam capacity race (overfill past size) | med | confirmed |
| DS-1 | gold-star | `/give_star` RMW mint/loss/free-cap bypass | med | confirmed |
| DS-1 | config (zayden-app) | entitlement `grant` cache downgrade | med | confirmed-logic / plausible-impact |
| DS-1 | temp-voice | claim/transfer leaves old owner's perms | med | confirmed |
| DS-2 | temp-voice | claim RMW race → stray owner grants | low | confirmed |
| DS-5 | gambling | `bet` has no balance floor → overdraft | med | confirmed-guard / plausible-interleave |

### Deep-sweep index — second pass (2026-07-17)

A repeat sweep drilled the modules the first pass left unexamined (levels,
family, ticket, suggestions, music, reaction-roles, verify, palworld) rather than
re-covering the economy RMW cluster. New confirmed defects:

| ID | Module | Class | Severity | Confidence |
|----|--------|-------|----------|------------|
| DS-1 | family | `/block` never enforced + `/unblock` never deletes → feature inert | med | confirmed |
| DS-2 | family | marry/adopt accept re-checks no invariant → `MAX_PARTNERS`/parent bypass | low | confirmed |
| DS-1 | suggestions | flipped `neg-pos` demote threshold → downvoted posts never removed; per-reaction full-channel scan | med | confirmed |
| DS-1 | ticket | `/support list` builds >25 select options → 400 past 25 FAQ msgs | med | confirmed |
| DS-2 | ticket | `/lfg tags` emits empty select menu (0 options) → 400 | low | confirmed |
| DS-1 | music | `VoiceOccupancy` keyed by `UserId` only → premature auto-disconnect (multi-guild) | low-med | confirmed |
| DS-6 | gambling | lotto `WeightedIndex` rebuilt after final pick → whole draw rolls back at exactly 3 participants | med | confirmed |

**Off-theme cluster this pass:** unlike the first sweep's RMW theme, these are
**boundary/limit and dead-feature** defects — three break at a specific size
(exactly 3 lotto players; >25 FAQ messages; fully-tagged thread), two are silent
dead features (family block/unblock; suggestions demote), and two are aliasing /
stale-state (music occupancy; marry/adopt accept). Two candidates were traced and
**dropped as unreachable**: `craft`/`sell` `cost * amount` overflow (max recipe
cost 500 × Discord's 2^53 integer cap < `i64::MAX`) and the family self-adopt path
(blocked by the `family_parent_child CHECK (parent_id <> child_id)`).

### Deep-sweep index — third pass (2026-07-17)

A further sweep drilled modules the first two passes labelled "essentially clean"
and skipped (marathon, the cron scheduler, levels XP accrual, destiny2 tierlist).
Most candidates traced clean — see below — but one new confirmed defect:

| ID | Module | Class | Severity | Confidence |
|----|--------|-------|----------|------------|
| DS-1 | marathon | `consensus` tiebreak non-deterministic when ≥2 sources collapse to `rank == len` (weapon/runner `description` flips across refreshes) | low-med | confirmed |

**Traced clean this pass (recorded so re-audits don't re-walk them):**
- **Cron scheduler** (`bot/src/cron.rs`) — the "only the earliest-tied jobs run"
  shape *looks* like it would starve low-frequency jobs behind the every-10-min
  `stamina` job, but it does not: a job whose fire time falls strictly between the
  frequent job's slots becomes the strict earliest and runs, and a job aligned to a
  slot ties and runs alongside. The registered schedules (all second-`0`, minute-
  aligned) are all reachable. The `M9-correctness` TODO's `t > now` / `includes(t)`
  redundancy is real but harmless.
- **levels XP cooldown** (`message_create.rs` + `bot/src/bindings/levels/mod.rs`) —
  `FullLevelRow::new_message` never touches `last_xp`, which *looks* like the 1-min
  cooldown can never advance, but `save`'s SQL sets `last_xp = now()` unconditionally,
  so the cooldown holds. (The read-modify-write can still be double-counted by two
  same-tick messages, but that is the known CC-9 class on a self-only, low-value row.)
- **destiny2 `tierlist` archetype autocomplete** (`endgame_analysis/tierlist.rs:153`)
  — no `.take(25)` cap, unlike palworld/marathon, but `Weapon::archetype` collapses
  to the ~20 distinct weapon *types* (`weapon.rs:192`), under Discord's 25-choice
  limit, so no 400.

### Deep-sweep index — fourth pass (2026-07-17)

A further sweep drilled the modules the first three passes never opened
(dashboard auth/OAuth/Ko-fi, destiny2 loadout render, palworld upload, reaction-
roles). The economy RMW theme is exhausted; this pass's finds are an **async
lock-across-await + ack-timeout** on the destiny2 build renderer and a
parsing panic. New defects:

| ID | Module | Class | Severity | Confidence |
|----|--------|-------|----------|------------|
| DS-1 | destiny2 | `/builds` holds `RwLock<BotState>` **write** guard across emoji upload+`sleep(5s)`×10 (≤50s) → global BotState stall; also no `defer` → 3s ack timeout | high | confirmed |
| DS-2 | destiny2 | `compendium::update` `swap_remove(2)` panics on a <3-cell "gear perks" row → refresh aborts, perk cmd stays broken | low-med | plausible |

The destiny2 DS-1 is a genuinely new **class** for this workspace: a
`tokio::sync::RwLock` (not std/parking_lot) held across an `.await` that includes
network I/O and `tokio::time::sleep`. The `clippy::await_holding_lock` gate does
not catch tokio locks, so it passed lint and the first-pass audit's "no locks
across `.await`" line — the `#[expect(clippy::significant_drop_tightening)]` on
`into_component` even documents the guard being held deliberately. Worth grepping
the other `data.write().await` sites (`state.rs:150`, music) for the same shape.

**Traced clean this pass (recorded so re-audits don't re-walk them):**
- **Dashboard authz** — `guild_admin_context` (`server/auth.rs:64`) gates every
  settings/module write on the caller's OAuth-reported `ADMINISTRATOR |
  MANAGE_GUILD` for that exact guild; the OAuth `state` CSRF cookie is validated
  (`web/routes_login.rs:41`); Ko-fi webhook checks `verification_token`. No IDOR.
  Minor: the `session_cache` (1-min TTL, `main.rs:75`) serves cache hits without
  re-checking `expires_at`, so a session that expires server-side is still honored
  for ≤60 s — bounded and low-value (session TTL is 7 days).
- **Palworld upload** — `.sav` extension + size (`content_length` *and* streamed
  body) checked, save I/O in `spawn_blocking`, atomic write; per-user cooldown
  re-checked at submit. The select→upsert cooldown window is a bypass but low-value
  (per-user, no economy).
- **destiny2 autocomplete caps** — perk `search` has `LIMIT 25` (`db/compendium.rs:30`),
  loadout autocomplete `.take(25)` (`loadouts/mod.rs:200`); no >25-choice 400.

### Deep-sweep index — fifth pass (2026-07-17)

A further sweep re-read the **music playback state machine** (previously only its
occupancy cache was examined) plus a spread of quick-verify lenses across
`family` (graph traversal), `palworld` (breeding pathfinder), `music` (queue
bounds / skip generation), and migration up/down pairs. One new confirmed defect —
the workspace's second **double-submit / missing-idempotency** find, this time
*outside* the economy layer:

| ID | Module | Class | Severity | Confidence |
|----|--------|-------|----------|------------|
| DS-2 | music | concurrent first-`/play` double-start → overlapping audio + orphaned uncontrollable `TrackHandle` + double queue-advance | med | confirmed |

The music DS-2 is the mirror image of the gambling wager-game result: gambling
*closed* intra-session double-submit with the atomic `GameCache::check_and_set`,
but the music "start if idle" transition (`enqueue` releases the player lock
between the `current.is_none()` check and `start_playback`, and `generation` is
never bumped on the enqueue path) has **no** equivalent guard, so two same-tick
play-family interactions both `play_input`. See [music.md DS-2](music.md).

**Traced clean this pass (recorded so re-audits don't re-walk them):**
- **family `tree` recursion** (`bot/src/bindings/family.rs:134-199`) — the
  "already in tree" guard (`:146`) is checked *before* each node is inserted
  (`:158`) and recursion only proceeds on freshly-inserted nodes, so an
  adoption/partner **cycle** (reachable via the family DS-2 invariant bypass)
  terminates rather than infinite-looping or stack-overflowing.
- **music `Queue::move_song`** (`queue.rs:67-74`) — validates `to` against the
  pre-removal length, but after `remove(from)` the new length is `old_len - 1`, so
  the validated `to ≤ old_len-1 = new_len` stays within `VecDeque::insert`'s legal
  `0..=len` range; no panic.
- **music `skip` generation handling** (`skip.rs:53-93` + `player.rs:48-74`) —
  `advance_queue` calls `advance()` which bumps `generation`, and `skip` reads
  `guard.generation` *after* that, so stopping the old handle fires a
  `TrackEndNotifier` whose stale generation fails the `generation == generation`
  guard: no double-advance on skip. (This is exactly the guard that the *enqueue*
  path lacks — hence DS-2.)
- **music teardown** (`disconnect.rs:17-18`, `control_panel.rs:113-114`) — both
  call `voice::leave` **before** `music.remove`, so no lingering songbird call /
  connection leak.
- **palworld `BreedingIndex::plan`** (`breeding.rs:66-188`) — the AND-dependency
  shortest-path relaxes each breeding hyperedge from *both* parent endpoints
  (`incident` built symmetrically, `:86-95`) and only when the partner is already
  `finalized` (`:128`), so the edge fires when the later parent finalizes with both
  ready; self-breed (`a == b`) finalizes the parent before scanning its incident
  list. `reconstruct` is `MAX_RECONSTRUCT_OPS`-bounded (`:302`).
- **migration up/down pairs** — spot-checked `0003_settings_split`,
  `0009/0011_palworld`, `0013_rename_enterprise_to_ultra`; downs reverse the
  schema. (`0003.down` and `0013` drop data that the up-direction introduced, which
  is inherent to a rollback, not an asymmetry defect.)

### Deep-sweep index — sixth pass (2026-07-17)

A further sweep drilled the **binding/glue layer** (`bot/src/handler`,
`bot/src/bindings`) and the modules the first five passes never opened
(reaction-roles event handler, verify, ticket close, levels/gambling pagination).
The finds are an atomicity gap in the *wiring* between a module and its reward, and
a dead-feature/doc-correction — both invisible to per-crate audits because they
live in the glue, not in any one crate.

| ID | Module | Class | Severity | Confidence |
|----|--------|-------|----------|------------|
| DS-1 | bot | level-up coin reward is a second tx after XP commits → co-future error in `message_create` try_join drops the reward permanently | med | confirmed |
| DS-2 | bot | `bindings/moderation/*` is orphaned (never `mod`-declared) → `/infraction`,`/logs`,`/rules` dead feature; corrects CC-3 + bot.md Clean §#2; harbors 3 latent bugs (mute→Ban mislabel, reachable `unreachable!` on `points≤0`, ban-blocked-by-closed-DMs) | med | confirmed (dead-feature) / latent (sub-bugs) |
| DS-1 | reaction-roles | handler has no bot-reaction filter → seeding a panel grants the reaction-role to the bot itself | low | confirmed |
| DS-7 | gambling | `daily`/`work` are further CC-9 whole-row absolute-overwrite sites (lost concurrent update; date guard blocks double-credit) | low-med | confirmed-logic / plausible-interleave |

**Correction to the baseline (important):** CC-3's site
`bot/src/bindings/moderation/infraction.rs:210` and bot.md's Clean §#2 both treat
the `moderation` binding as compiled/live. It is **not** — the directory is not
`mod`-declared in `bindings/mod.rs` and references a non-existent `core::SlashCommand`
trait, so it never builds. See [bot.md DS-2](bot.md). Fix or delete the tree and
drop the CC-3 entry.

**Traced clean this pass (recorded so re-audits don't re-walk them):**
- **levels/gambling leaderboard pagination** — both clamp the page (`.max(1)` /
  `(page-1).max(1)`) before computing `OFFSET (page-1)*10`, so no negative-offset
  SQL error; "next"-past-end returns an empty page guarded by an `is_empty()` error
  reply (gambling) or a blank embed (levels), no panic.
- **ticket `close`** (`slash_commands/ticket/close.rs`) — rename-only, no
  open/closed DB state, so no illegal-transition invariant to break; truncates the
  new name to 100 chars.
- **verify `Panel`** — click-to-verify grants a hardcoded role with no captcha (by
  design); only notable smell is the guild-scoped `VERIFIED_ROLE` const behind a
  global `"verify"` `custom_id` (harmless in a single-guild deploy).

### Deep-sweep index — seventh pass (2026-07-17, production-log-driven)

Production error/warning logs were supplied and each was traced to its code path.
This pass is the highest-signal of all — every entry is a **reproduced-in-prod**
defect, not a hypothesised one. Two confirm earlier findings; two are new.

| ID | Module | Class | Severity | Confidence |
|----|--------|-------|----------|------------|
| DS-8 | gambling | stamina cron `UPDATE` has **no `WHERE`** → full-table rewrite every 10 min → `40P01` deadlock with gameplay writes + >1 s slow statement + table bloat | high | confirmed (prod) |
| DS-6 | gambling | lotto `WeightedIndex` rebuilt after final pick → "Not enough weights > zero" → draw rolls back | med | confirmed (prod) |
| DS-3 | destiny2 | endgame sheet parse failures (`Frame::from_str` drift: bare `Dynamic`/`Balanced`; `perk 1 cell value`) silently drop weapons, and `TRUNCATE`-replace makes it destructive → tierlist/perk data erodes | med | confirmed (prod) |
| DS-2 | destiny2 | compendium `swap_remove(2)` short-row panic — same "sheet drift breaks parser" family as DS-3 | low-med | plausible → now corroborated by DS-3 |

**Log → finding map (for traceability):**
- `stamina cron update failed | … code: "40P01", message: "deadlock detected" …
  relation "gambling"` and `slow statement … UPDATE gambling SET stamina =
  LEAST(stamina + 1, $1) … rows_affected=0 elapsed=1.003s` → **[gambling DS-8](gambling.md)**
  (`bot/src/bindings/gambling/stamina.rs:12-19`, WHERE-less full-table update).
- `lotto cron job failed: … WeightedIndex update failed: Not enough weights > zero`
  → **[gambling DS-6](gambling.md)** (`games/lotto.rs:114-117`, rebuild after final
  removal on an empty list).
- `Failed to parse: 'Dynamic'` / `'Balanced'`, `Skipping weapon build in '…':
  missing data: frame parse`, `Skipping weapon in 'Swords': missing data: perk 1
  cell value` → **[destiny2 DS-3](destiny2.md)** (`sheet/frame.rs:42-84` +
  destructive `db/endgame.rs:91` TRUNCATE-replace).

**Theme:** the prod logs cluster on **cron/batch jobs** (stamina regen, lotto draw,
endgame refresh) — the unattended paths with no user watching a response, where a
silent failure or a full-table lock goes unnoticed until it deadlocks or a feature
quietly empties out. The gambling `stamina` table is also the same hot row-set the
CC-9 economy races contend on, so DS-8's WHERE-less churn actively *widens* those
windows. Recommend auditing every remaining `CronJob` action for (a) unbounded
/ WHERE-less writes and (b) all-or-nothing error handling that discards a whole
tick on one bad row.

### Deep-sweep closing note

Nine single-lens passes were run across the workspace (silent-failure,
concurrency/atomicity, Discord-API correctness, SQL integrity, numeric/boundary,
resource lifecycle, state-machine, input/authz, duplication/drift), followed by
combined-lens re-sweeps of the high-signal modules (`gambling`, `lfg`,
`temp-voice`, entitlement, `music`, voice cache).

**Where the deep defects clustered:** overwhelmingly in the **check-then-act /
read-modify-write** shape (CC-9) on economy and counter rows. Every confirmed
mint/loss/limit-bypass (gambling `/send`, `/gift`, prestige→lotto; gold-star
`/give_star`; lfg fireteam capacity) reduces to the same root: a value read in one
statement, mutated in memory, and written back with an **absolute** upsert or an
unguarded decrement, with no transaction spanning the guard and the write. The
CC-1 generic-manager split is what camouflages it — the racy read lives in the
module crate while the write semantics (`EXCLUDED.col` vs `col + $n`) live in
`bot/src/bindings/*`, so no single file exposes the hazard. Two off-theme finds:
a real `ON CONFLICT` arithmetic bug (`lotto.sql`, DS-3) and a cache-aggregation
asymmetry in entitlement `grant` (config DS-1).

**What was verified clean under these lenses:** wager-game intra-session
double-submit (guarded by `GameCache` + atomic `bet`), the temp-voice occupancy
count ordering (update-before-count; double-delete is idempotent), and the LFG
join *capacity check itself* (the flaw is missing serialization, not the check).
The single highest-value remediation is to convert the CC-9 `save` sites to
single-transaction conditional writes — ideally folded into the CC-1
concrete-`PgPool` migration, since both touch the same methods.

**Sixth-pass addendum:** re-focusing on the *binding/glue* layer (rather than any
one crate) surfaced two defects the crate-scoped passes structurally could not see:
the level-up reward atomicity gap (XP committed in the module, reward wired in the
`bot` handler as a second tx — [bot.md DS-1](bot.md)) and the orphaned `moderation`
tree that the baseline mistakenly recorded as live ([bot.md DS-2](bot.md)). The
lesson mirrors CC-9's: the highest-risk defects sit in the seams *between* a module
and its `bot/src/bindings`/`handler` wiring, where no single file — and no
single-crate audit — shows the whole hazard.

## Clean (verified workspace-wide)

- No `todo!()` / `unimplemented!()` (workspace lints deny them; grep confirms
  none).
- No blocking `std::fs` on async hot paths **except** `llamad2` (see
  [llamad2.md](llamad2.md)); `palworld` correctly wraps save I/O in
  `spawn_blocking`, and `zayden-app`/`bot` `std::fs` is startup-only config load.
- `unwrap()`/`expect()` on live paths is rare (only `bot` has a handful, 6).
- No locks held across `.await` in the async-heavy crates: `music` uses
  `tokio::sync::Mutex` intentionally and `DashMap` for the players map.
- `cargo machete` / unused-dep hygiene is maintained per the milestone exit
  gates (re-verify at fix time).
