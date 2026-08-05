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
- **Status:** `complete — per-module, see below`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Reconciled (2026-07-29):** No fix was performed by this task — CC-1 was
  **already fully closed** by the per-module migrations that each cited it, and
  never had its umbrella status tag set. All eight enumerated modules went
  concrete:

  | Module | Commit |
  |--------|--------|
  | `gambling` | `83930148` |
  | `family` | `5ac30447` |
  | `lfg` | `240b47e5` |
  | `temp-voice` | `611d350b` |
  | `levels` | `04a8ab2b` |
  | `reaction-roles` | `c7c535de` |
  | `suggestions` | `b4bb8582` |
  | `gold-star` | `c2b4c4cf` |

  **Verified against `bf0d90ff`:** a workspace grep for `<Db`, `Db:`,
  `Database>`, and `Pool<` (non-`PgPool`) returns **zero** hits in `src/` — the
  only matches are a doc-comment in `family/tests/manager.rs` describing the
  migration, and two unrelated field initialisers (`MarathonDb`, `PalDb`).
  `levels/src/sqlx_lib.rs` is gone, `gold-star/src/manager.rs` is a plain
  `PgPool` + `FromRow` module, and `zayden-core/src/` carries no manager traits.
  `async-trait` remains a dependency of only `bot`, `zayden-core`, `music` and
  `zayden-app`; all eight module crates dropped it.
- **Not part of this finding (correctly left alone):** the surviving
  `#[async_trait]` sites are the `ModuleCommand` / `ModuleComponent` /
  `ModuleModal` / `ModuleAutocomplete` impls in `bot/src/bindings/*`, declared in
  `zayden-core/src/module.rs`. Those **must** stay boxed — they are consumed as
  trait objects (`Arc<dyn ModuleCommand>`, `DispatchMap<dyn ModuleComponent>` at
  `bot/src/registry/mod.rs:31-34`), which native `async fn` in traits does not
  support. They are the manual serenity routing framework `CLAUDE.md` mandates,
  not the DB-generic manager pattern this finding describes. Likewise the
  non-DB `async_trait` traits in `music/src/resolve/` and
  `zayden-app/src/entitlement/provider/`.
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
- **Status:** `complete — de1238d8`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-24):** Relocated the **7 surviving** inline `#[cfg(test)]` modules
  (the original list's `gambling/components/tictactoe.rs` and `family/family_manager.rs`
  sites were already deleted/renamed by the CC-1 migrations) to `tests/` integration
  files, exposing the minimum surface for each:
  - `zayden-core/src/snowflake.rs` → `zayden-core/tests/snowflake.rs` (fns already `pub`).
  - `zayden-app/src/entitlement/types.rs` → appended to `zayden-app/tests/entitlement.rs`
    (`EntitlementScope` API already `pub`).
  - `temp-voice/src/voice_channel_manager.rs` → `temp-voice/tests/voice_channel_row.rs`
    (`VoiceChannelRow` API already `pub`).
  - `temp-voice/src/commands/mod.rs` (`has_manage_channels`) → `temp-voice/tests/permissions.rs`
    (made the fn `pub`, reachable via `commands::has_manage_channels`).
  - `family/src/commands/information/siblings.rs` (`collect_sibling_ids`) →
    `family/tests/siblings.rs` (made the fn `pub`, re-exported via `commands`).
  - `palworld/src/commands/breed_plan.rs` (`gender_gap`) → `palworld/tests/gender_gap.rs`
    (made the fn `pub`, re-exported via `commands`; the now-public-API `implicit_hasher`
    lint required generalising its `HashMap` over `BuildHasher`).
  - **`bot/src/registry/dispatch_map.rs`** (the flagged no-lib-target case): moved
    `DispatchMap`/`OverlapError` **into `zayden-core`** (`src/dispatch_map.rs`, re-exported
    from its `lib.rs`) — they are generic routing infra whose only dependency,
    `IdMatch`, already lives there. `bot::registry` now imports `zayden_core::DispatchMap`
    and re-exports `OverlapError` (bindings' `crate::registry::OverlapError` path
    unchanged). Test → `zayden-core/tests/dispatch_map.rs`.
  All 35 relocated tests pass. Gate green: `cargo +nightly clippy --workspace
  --all-targets -D warnings` clean, `cargo test` green, no new `#[allow]`/`#[expect]`.
  No SQL / `Cargo.toml` dep change, so no `.sqlx`/`machete` delta. **Residual:** none for
  CC-2; the `DispatchMap` relocation makes a `bot` lib target unnecessary.
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
- **Status:** `complete — 3d787146`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-28):** Triaged **all** sites. The inventory had grown from 22 to
  **27** by fix time (the CC-1 migrations moved SQL into the module crates and
  carried their suppressions with them — the `bot/src/bindings/*` sites in the list
  below are gone, replaced by sites in `gambling`/`levels`/`lfg`/`temp-voice`).
  **10 suppressions removed, 17 remain**, all of the remaining now carrying a
  `reason` that states a real invariant rather than a deferral.
  **Corrected 2026-07-29: 9 removed, 18 remain** — the `RIGGED_LUCK` deletion
  claimed below never landed (see the struck-through entry). The live inventory
  on `bf0d90ff` is **24** attributes, which reconciles exactly:
  18 (CC-3 survivors) + 6 added *after* this fix by the palworld save-editor
  work (`palworld/src/progress/mod.rs` ×5 and `palworld/tests/progress.rs` ×1,
  all introduced with the file in `0f309783`, 2026-07-28 — a legitimate new
  addition, not a CC-3 regression). Spot-checked and **confirmed landed**: the
  other 9 removals (`GameEmbed<'a>` in `utils.rs:85`, `GIFT_AMOUNT` in
  `gift.rs:35`, `prize_share: &[i64]` in `lotto.rs:101`, the one-field
  `LeaveInteraction` in `leave.rs:17`, and the rest).

  **Removed by fixing the cause (10):**
  - `gambling/src/utils.rs:85` `too_many_arguments` → the 9-arg `game_embed` free
    fn became a `GameEmbed<'a>` struct with a `build(&EmojiCache)` method
    (the audit's prescribed "bundle args into a struct"); the two call sites
    (`coinflip`, `roll`) now use struct literals. The `impl Into<GameResult>`
    ergonomics moved to `.into()` at the call site.
  - `gambling/src/models/mod.rs` `cast_sign_loss` on `Stamina::stamina_str` →
    `usize::try_from(...).unwrap_or(0).min(max)`. **This one was masking a live
    panic:** nothing guarantees stamina is non-negative (`done_work` decrements
    unconditionally), and `-1 as usize` asks `str::repeat` for ~2^64 copies →
    `capacity overflow` abort. Verified fails-before (see Verification).
  - `gambling/src/commands/gift.rs` `cast_possible_truncation` +
    `cast_precision_loss` → `const GIFT_AMOUNT: i64 = START_AMOUNT * 5 / 2`
    replaces the `(START_AMOUNT as f64 * 2.5) as i64` float round-trip. Same value.
  - `gambling/src/games/lotto.rs` `cast_possible_truncation` +
    `cast_precision_loss` → **float money removed from the economy**:
    `select_winners`'s `prize_share` is now `&[i64]` in whole percent
    (`[50, 30, 20]`, was `[0.5, 0.3, 0.2]`) and the payout is
    `i64::try_from(i128::from(jackpot) * i128::from(share) / 100)`, widened so the
    multiply cannot overflow. `tests/lotto.rs`'s exact-payout assertions
    (200k/300k/500k) pass unchanged, pinning the value equivalence.
  - ~~`gambling/src/common/shop/items.rs:192` `dead_code` → deleted the unused
    `RIGGED_LUCK` const ("reserved for future implementation"). This is the
    [CC-4](#cc-4) sub-item that lived in this file.~~
    **CORRECTION (2026-07-29, CC-1 reconcile task): this claim is false — the
    deletion never happened.** The `#[expect(dead_code)]` and the `RIGGED_LUCK`
    const are both still at `items.rs:192-201` on `bf0d90ff`, and
    `git log -S RIGGED_LUCK` on that file shows `3d787146` never touched it.
    So **9 suppressions were removed here, not 10**, and the CC-4 sub-item that
    lives in this file is still open. Tracked in [CC-4](#cc-4).
  - `levels/src/manager.rs` `cast_possible_truncation` → `FullLevelRow::save`
    binds `i32::try_from(self.total_xp).unwrap_or(i32::MAX)` (same for
    `message_count`) instead of `as i32`, so an over-large counter **saturates**
    at the INT4 ceiling rather than wrapping to a negative XP total.
  - `family/src/commands/tree.rs` `cast_precision_loss` → the two `usize as f64`
    layout casts widen through `u32::try_from(..).unwrap_or(u32::MAX)` +
    `f64::from`, which is lossless.
  - `music/src/embeds.rs:50` `cast_possible_truncation` + `cast_sign_loss` →
    `progress_bar`'s fill position is now integer round-half-up on nanoseconds
    (`(2·W·elapsed + total) / (2·total)`) instead of
    `(ratio * WIDTH).round() as u32`. Pinned equivalent to the float version at
    every second of a track by a new test.
  - `lfg/src/actions/leave.rs:19` `dead_code` → `LeaveInteraction` had `author`
    and `user` fields that no caller ever read: **every** call site passes the
    target user as a separate argument, and `/lfg leave` registers no target
    option at all, so the `From` impls' `guardian`-option and `UserSelect`
    parsing was dead by construction. Reduced the struct to the one consumed
    field (`thread`) and deleted the dead parsing.
  - `lfg/src/cron/reminders.rs:20` `significant_drop_tightening` → dropped the
    intermediate `let jobs = data.jobs_mut()` binding that forced the write guard
    to stay live, and hoisted the `format!("lfg_{post_id}")` out of the `retain`
    closure (it was re-allocating per element).

  **Kept, with the reason sharpened (17):**
  - **The 8 `trivial_casts` sqlx sites** (`gambling/commands/{dig,goals,work}.rs`,
    `lfg/models/post.rs` ×2, `temp-voice/voice_channel_manager.rs`,
    `zayden-app/entitlement/service.rs`, `dashboard/web/routes_login.rs`) are
    **load-bearing, not escape hatches** — this is the pass's main correction to
    the finding. `expr as T` on a bind argument is *sqlx's type-override syntax*,
    not a Rust cast; removing it fails the build with `optional sqlx feature
    "time" required for type TIMESTAMPTZ of param #N`, because `jiff_sqlx` types
    have no built-in mapping. Verified empirically by removing all 8 and
    compiling. What *was* wrong is scope: 6 sat on the whole `fn`, where they
    would also silence an unrelated real cast. All are now narrowed to the single
    `let`/statement holding the macro, and the `reason` says why it is not a cast.
    (The two fn-level survivors — `PostRow::edit` and the tail of the others —
    are fns whose body *is* the one macro call.)
  - **The 7 `const fn` builder sites** (`destiny2/raid_guides/mod.rs` ×6 =
    3 × `indexing_slicing` + `panic` pairs, `gambling/common/shop/items.rs:47`)
    document a genuine **compile-time** invariant: these run in const context, so
    a violated slot count is a build error, not a runtime panic. Left as-is —
    destiny2 #1 (retire the all-`const` render pipeline) is the real fix target.
  - `destiny2/endgame_analysis/sheet/tier.rs:104` `cast_possible_truncation` +
    `cast_sign_loss` — std has no fallible float→int conversion, so an `f64 → u8`
    cast is unavoidable; the `clamp(0.0, 1.0) * 255.0` is what makes it total.
    Reason reworded to say that.
  - `bot/src/handler/mod.rs:121` `wildcard_enum_match_arm` — `FullEvent` is large
    and `#[non_exhaustive]`; the filter fn is the real exhaustiveness gate.
- **Where (22 sites, as inventoried 2026-07-17):** `bot/src/handler/mod.rs:121`,
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
- **Residual / follow-ups:**
  - **[CC-4](#cc-4) is now only half-closed.** Its `items.rs:192` `dead_code`
    sub-item was deleted here, but its main subject — the `tictactoe` `GameState`
    stub and the `#[expect(clippy::future_not_send)]` at
    `gambling/src/commands/tictactoe.rs:175,182` — **no longer exists in the tree**
    (the file has no `#[expect]` left). CC-4 should be reconciled on its own.
  - The 8 sqlx `trivial_casts` suppressions can only be retired upstream, by
    `jiff_sqlx` gaining a built-in TIMESTAMPTZ mapping (or sqlx growing a
    non-`as` param-override syntax). Not actionable here; recorded so a future
    pass does not re-litigate them.
  - `destiny2/src/loadouts/record.rs:89` from the original list no longer carries
    a suppression (removed by an earlier task); the inventory above is corrected.

### CC-4. `tictactoe` dead `GameState` stub  ·  #2  ·  low
- **Status:** `complete — a2c6f652`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Reconciled (2026-07-29):** the marker below was left at `in-review` after the
  human committed the fix; `a2c6f652` ("remove dead code for RIGGED_LUCK and
  update audit findings") is that commit, so the status is now `complete`.
  Its follow-up, [gambling #2b](gambling.md) (`WEAPON_CRATE`), was subsequently
  ruled **`wontfix`** — reserved shop items are planned features and are deleted
  or commented out only when an `#[expect]` flags them. That ruling does **not**
  reopen CC-4; it sets the policy for anything like it found from here on.
- **Fix (2026-07-29):** Closed. The reconciliation below reopened this finding
  after establishing that CC-3 never deleted `RIGGED_LUCK`; the follow-up task
  deleted it for real — the const (`gambling/src/common/shop/items.rs:192-201`),
  its `#[expect(dead_code)]`, and its commented-out `SHOP_ITEMS` entry
  (`items.rs:396`). Both halves of CC-4 are now gone from the tree.
  **Safe to delete:** `git log -S '    RIGGED_LUCK,'` on that file returns no
  commit, so the item was never uncommented into `SHOP_ITEMS`, never
  purchasable, and no inventory row can reference `"riggedluck"`.
  **No regression test** — deleting an unreferenced private const has no runtime
  behaviour to pin, and a "not in `SHOP_ITEMS`" assertion would have passed
  *before* the fix too (the entry was already commented out), so it could not
  fail-before. The compiler is the check: the const was private and unreferenced,
  so any surviving reference would break the build, and because `#[expect]` errors
  when *unfulfilled*, `-D warnings` would have caught a still-needed suppression.
- **Follow-up recorded:** deleting `RIGGED_LUCK` surfaced
  [gambling #2b](gambling.md) — `pub const WEAPON_CRATE` is the identical dead
  stub, but carries no `#[expect]` because `pub` + a fully public module chain
  makes rustc treat it as reachable API. Same defect, invisible to the gate.
  Left for its own task per one-finding-one-task.
- **Reconciled (2026-07-29):** CC-3's residual note asked for CC-4 to be
  reconciled on its own. Doing that turned up a **factual error in CC-3's own
  fix record**, so this finding is *not* closeable as written:
  - **`GameState` half — closed by `83930148`.** The stub and both
    `#[expect(clippy::future_not_send, reason = "dead code within GameState stub")]`
    attributes are gone from the tree, and `GameState` now returns zero hits
    workspace-wide. It was not deleted deliberately: the stub was itself
    DB-generic (`struct GameState<Db: Database, Manager: GameManager<Db>>`), so
    it died *with* the gambling [CC-1](#cc-1) migration. CC-3 was right that it
    "no longer exists in the tree", but attributed it to no commit.
  - **`items.rs` half — still live. CC-3's record is wrong here.** CC-3's fix
    note claims `gambling/src/common/shop/items.rs:192` `dead_code` was
    "**Removed** by fixing the cause … deleted the unused `RIGGED_LUCK` const".
    It was not. `#[expect(dead_code, reason = "reserved for future
    implementation")]` and the `RIGGED_LUCK` const are **both still present** at
    `items.rs:192-201`, with the item still commented out of `SHOP_ITEMS`
    (`items.rs:396`). `git log -S RIGGED_LUCK` on that file returns only the
    original `78edaf90` — `3d787146` never touched it.
  - **Why the gate never caught it:** the const genuinely *is* unused, so the
    `#[expect(dead_code)]` is satisfied and
    `clippy --workspace --all-targets -D warnings` stays green. A stale
    suppression that is still doing its job is invisible to the gate — only
    reading the record against the tree finds it.
- **Remaining work:** delete `RIGGED_LUCK` (`items.rs:192-201`) and its
  commented-out `SHOP_ITEMS` entry (`items.rs:396`), per the finding's own
  "delete the dead stub" direction and the mandate's preference for the correct
  end state over the low-churn path. One-crate change, `-p gambling`.
- **Where:** ~~`bot-modules/gambling/src/commands/tictactoe.rs:175,182`
  (`#[expect(clippy::future_not_send, reason = "dead code within GameState stub")]`)~~
  — removed in `83930148`; **still live:** the
  `#[expect(dead_code, reason = "reserved for future implementation")]`
  at `bot-modules/gambling/src/common/shop/items.rs:192`.
- **What:** Self-described dead/stub code retained behind `#[expect]`.
- **Why it matters:** Checklist #2 — soft stubs that compile but do nothing. They
  carry maintenance weight and their `#[expect]`s inflate CC-3.
- **Suggested fix:** Delete the dead stub, or wire it up. The mandate ("optimize
  for the correct end state, not the low-churn path") favours deletion until the
  feature is actually built.

### CC-5. Runtime `sqlx::query(...)` bypassing compile-time macros  ·  #1  ·  med
- **Status:** `complete — 6049775d`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-24):** Converted every remaining runtime-SQL site to compile-time
  macros. The audit's `gold_star.rs:83` site is already gone (folded into gold-star's
  CC-1 migration), but a full re-grep found the theme had **grown to 14 sites in 6
  files across 2 crates**, all now converted:
  `zayden-app/src/entitlement/service.rs` (6: `revoke_all_by_scope` DELETE,
  `revoke` DELETE…RETURNING, `refresh_expired_cache_rows` SELECT, `load_tier_from_db`,
  `aggregate_tier_from_db` MAX, `refresh_cache_row` upsert),
  `zayden-app/src/config/bot_config.rs:234` (`DbConfigRow` SELECT — dropped its now-dead
  `#[derive(sqlx::FromRow)]`), and dashboard `middleware/auth.rs`, `server/auth.rs` (×3),
  `server/tier.rs`, `server/guild.rs`, `web/routes_kofi.rs`. Single-column reads use
  `query_scalar!`, multi-column use `query!`/`query_as!` — matching the pre-existing
  `routes_kofi.rs` idiom; the untyped `.get::<T,_>("col")` / `sqlx::Row` accessors and
  their imports were removed. **`.sqlx`:** regenerated with
  `cargo sqlx prepare --workspace -- --all-features` against a throwaway **empty,
  freshly-migrated** Postgres 18 (12 new entries — 14 sites dedup to 12 distinct
  queries). Following the lfg #4 precedent, unrelated pre-existing drift the full
  regen surfaced was reverted so the diff is CC-5-only (see **Residual** below).
  Verified against that DB: `cargo +nightly clippy --workspace --all-targets -D warnings`
  clean, `-p dashboard --features ssr` clean, `cargo test` green. No new
  `#[allow]`/`#[expect]`. No test added — a runtime→compile-time-macro conversion has no
  runtime-behaviour delta; the guarantee *is* the build-time schema check (a wrong column
  now fails `cargo check` instead of at runtime).
  **Residual (pre-existing, not CC-5):** `cargo sqlx prepare --check` fails on **clean
  `main`** — the committed `.sqlx` is already drifted (missing/stale LEFT-JOIN entries:
  gambling `895e6b8`/`fc6caa8e`, lfg_posts `905f7d2` nullability). This predates and is
  independent of CC-5; worth its own finding (regenerate the whole cache against an empty
  DB), left untouched here.
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
- **Status:** `in-review — all 3 remaining crates worked (gold-star 9a7b8795; llamad2 b5cc3faf; verify wontfix)`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Closing pass (2026-07-31).** All three named crates are now resolved:
  [`gold-star`](gold-star.md#3-no-integration-tests--6--low) closed (`9a7b8795`),
  [`llamad2`](llamad2.md) closed (`b5cc3faf`), and
  [`verify`](verify.md) ruled **`wontfix`** — the judgement CC-6's earlier note
  asked to be made explicitly. Re-reading the crate confirmed the prediction:
  112 LOC, whose only branch (`src/lib.rs:64`) has an `add_role` call in one arm
  and an error variant in the other, so neither side is reachable without
  fabricating a serenity `ComponentInteraction`; the remainder is a constant
  embed and command metadata. No SQL, so the `#[sqlx::test]` harness the two
  earlier crates built does not apply. Full reasoning — including the one
  non-trivial property deliberately left untested (`Respond` never leaking a raw
  `serenity::Error`, which belongs in a workspace-wide finding, not a lone
  assertion in the smallest crate) — is in [verify.md #1](verify.md).
  **A `wontfix` closes a coverage finding legitimately when the crate has no
  independently observable logic** — the alternative is asserting a literal
  against itself, which the checklist #6 wording explicitly rejects. Record the
  reasoning per-item, as verify.md now does, so a re-audit does not re-open it.
  _(That pass also surfaced a genuine new defect the 2026-07-17 audit had marked
  clean: the hardcoded, duplicated `VERIFIED_ROLE` — [verify.md #2](verify.md),
  `open`. Reading a crate closely enough to rule on its testability is itself an
  audit; expect that.)_
- **The `Where` list below is now fully worked** (verified against the tree, not
  the record, per the workflow's 2026-07-29 lesson):
  `ticket` 1, `lfg` 5, `family` 4, `levels` 2, `reaction-roles` 2, `suggestions`
  2, `gold-star` 1, `llamad2` 2 test files; `verify` 0 by the ruling above. The
  two remaining entries are `bot` and `dashboard`, which the finding itself
  recorded as "no lib target, so integration tests are structurally awkward —
  noted, not blamed"; they were never in scope and are not a blocker to closing.
- **What the `llamad2` task adds to the pattern:** the two earlier crates were
  *whole crates* whose logic is SQL. `llamad2` is mixed, and splitting it by that
  line worked better than picking one harness: `tests/counters.rs` is DB-backed
  (`#[sqlx::test]`, no fixture — the empty table is the first case), while
  `tests/triggers.rs` is offline over three predicates. Two further notes:
  - **Reaching the SQL may need a small extraction.** `llamad2`'s upsert was
    inlined byte-identically in two `run()` methods taking `&Context`, so it was
    untestable in place. Extracting it to `Counter::bump` made it reachable *and*
    de-duplicated it. Keep the SQL string character-for-character identical and
    the `.sqlx` entry still resolves, so the cache needs no regeneration.
  - **Making private fns `pub` for a test trips `clippy::must_use_candidate`**
    (and `doc_markdown` on prose). Fix them — `#[must_use]`, backticks — do not
    reach for `#[expect]`; the same lint fired on the CC-2 relocations
    (`implicit_hasher` there) and is a normal cost of widening the surface.
- **This task changed how CC-6 can be closed — read before taking the next crate.**
  Every pre-existing `tests/` file in the workspace is **pure and offline**; the
  DB paths were consistently left uncovered with a note pointing back here
  (`gambling/tests/shop_buy.rs:13-18`, `levels/tests/accrual.rs:14-19`). For a
  crate whose logic *is* the SQL — which `gold-star` is, and which the remaining
  economy-adjacent crates are — offline tests can only assert constructor
  defaults and error strings, i.e. the trivia checklist #6 warns against. On the
  owner's ruling (2026-07-29) the harness was built instead:
  - `#[sqlx::test(migrations = "../../migrations", fixtures("<name>"))]` with
    `sqlx = { features = ["migrate"] }` on the crate. sqlx creates a fresh
    database per test, applies the workspace migrations to it, loads the fixture
    and drops it afterwards. No `sqlx migrate run`, no shared state between
    tests, and the `.sqlx` cache is untouched because fixtures are plain SQL
    applied at runtime rather than `query!` macros.
  - **Consequence for the gate:** `cargo test` now needs `DATABASE_URL` pointing
    at a Postgres the runner may create databases on. Use a throwaway server,
    never a live one — see [`CLAUDE.md`](../../CLAUDE.md) for the command. The
    build itself still works offline (`SQLX_OFFLINE=true`) from the committed
    cache.
  - **Verification pattern for a coverage finding:** a test written against
    already-fixed code cannot fail-before. Establish the equivalent by removing
    each guard in turn and re-running (then reverting), and record the matrix —
    including the guards that turn out **not** to be independently observable,
    rather than claiming coverage the suite does not have. `gold-star`'s
    `WHERE number_of_stars >= 1` was one such: redundant with the `FOR UPDATE`
    read in a single process.

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
- **Status:** `complete — c794fe8f`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Reconciled (2026-08-04):** the marker was left at `in-review` after the human
  committed the task as `c794fe8f`. Verified against the tree, not the record:
  `components/custom_id.rs` declares all four enums (`:7`, `:46`, `:76`, `:106`),
  `GamblingError::NotYourGame` exists (`error.rs:37`), and a grep for the old
  literal prefixes (`"ttt_`, `"bj_`, `"hl_`, `"prestige_`) outside `custom_id.rs`
  and `tests/` returns nothing.
- **Fixed 2026-08-04.** `bot-modules/gambling/src/components/custom_id.rs` adds
  `BlackjackCustomId`, `HigherLowerCustomId`, `PrestigeCustomId` and
  `TicTacToeCustomId`, each with `as_str`/`Display` + `FromStr`, following the
  `LevelsCustomId` precedent. All four routers now `parse::<…>()?` instead of
  matching literals, and all eleven `CreateButton::new` producers build their id
  from the enum — so producer and consumer are one source. Wire ids are
  unchanged, so messages already posted in Discord still route. Regression test:
  `bot-modules/gambling/tests/custom_id.rs`.
- **Not just hygiene — it was hiding a live defect.** `components/tictactoe.rs`
  routed cancel as `"ttt_cancel" if metadata.user == interaction.user`. A match
  guard that fails falls through to the *next* arm, so a **non-owner's cancel
  click** reached the coordinate catch-all, which stripped `ttt_` and tried to
  read `"cancel"` as a board position — surfacing as
  `Internal("row index not parseable")`, which has no `user_message` and so
  showed the clicker a generic failure. The ownership check now lives *inside*
  the `Cancel` arm and returns a new `GamblingError::NotYourGame`
  ("This isn't your game."). Recorded here rather than as its own `DS-#`: the
  fall-through is only expressible because the ids were untyped, so it is an
  instance of this class, not a separate one.
- **Two deliberate behaviour changes** beyond the rename, both making a silent
  no-op visible: the blackjack binding's `_ => ()` and the higher-lower `_ => {}`
  arms are gone, so an unroutable id in either namespace is now an error rather
  than a dropped interaction. Higher-lower's parse also moved **above** the
  deck-exhaustion branch that credits a gem, so an unroutable click can no longer
  mint one on its way to doing nothing.

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
     _(Both `setup` commands are now **removed** — lfg #4 at `51d8412e`,
     temp-voice #5 in review; the dashboard is the single editor of both tables.
     The remaining duplication sub-item is the support/channels/roles config
     commands.)_
  2. **Config still stranded in-bot.** `ticket` (support-guild config) and
     `suggestions` config have **no** dashboard equivalent yet, though they are
     the same shape as things already moved.
     _(`ticket` is now **done** — see [ticket.md #2](ticket.md). It sharpened the
     heuristic a third time: the two earlier splits divided a *live* editor
     between bot and web, but here neither side worked. The dashboard wrote
     `support_settings.support_role_id` while the bot read `guild_support_roles`,
     which nothing wrote — a duplicated *concept* with no duplicated write, which
     reads as "stranded config" until you check whether the reader and the writer
     name the same table. Worth applying that check to the remaining candidates
     before assuming a bot editor exists at all. The corrective pattern is CC-5's:
     one owner for the SQL — here `ticket::SupportRoles` — with the dashboard
     calling into the module crate rather than issuing its own statements.)_
     _(`reaction-roles` mapping CRUD is now **done** — see
     [reaction-roles.md #3](reaction-roles.md); it set the pattern for admin CRUD
     of *reference data* rather than a settings row: a table page plus server
     fns, with the module crate kept as the single owner of the SQL and of any
     value-normalisation contract the bot relies on. It also sharpened the
     heuristic a second time: music #3 split by **field**, reaction-roles splits
     by **operation** — an in-bot writer earns its keep when the Discord client
     supplies input the web cannot match, here the emoji picker. When a
     duplicated writer survives on those grounds, converge it on one SQL path
     and one normalisation rule rather than accepting two.)_
     _(`music` is now **split** — see [music.md #3](music.md). The owner's ruling
     refines the heuristic below: "one-shot config → dashboard" is really
     **"admin setup → dashboard, live-tweak → bot"**. Music's DJ role,
     auto-disconnect and now-playing announcements moved to `save_music_settings`;
     default volume, 24/7 and autoplay stayed on `/music settings` because they are
     changed *while music is playing*. Per-field ownership, not per-command — apply
     the same lens to the three remaining modules rather than moving whole
     commands.)_
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
  - `music` — **done (split)**: DJ role / auto-disconnect / announce-now-playing →
    dashboard (`save_music_settings`); default volume / 24-7 / autoplay stay in-bot
    alongside playback + control panel. See [music.md #3](music.md).
  - `ticket` — support-guild / panel config → dashboard. Keep open/close/claim
    interactions in-bot.
  - `suggestions` — channel/threshold config → dashboard. Keep the submit modal +
    vote reactions in-bot.
  - `reaction-roles` — **done (split)**: the browsable list of
    message→emoji→role maps and mapping *removal* moved to the dashboard
    (`/guild/:id/reaction-roles`); `/reaction_role add` **stays in Discord** on
    the owner's ruling — the client's emoji picker beats a web text field. The
    reaction event handler stays in-bot. See
    [reaction-roles.md #3](reaction-roles.md).
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

### CC-10. Committed `.sqlx` offline cache is drifted on `main`  ·  #1 / #7  ·  med
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-08-05).** Confirmed red, then regenerated. The human stood up a
  throwaway `postgres:18-alpine` on `:55432`, `sqlx migrate run` applied all **23**
  migrations to an **empty** `zayden_prepare`, and
  `cargo sqlx prepare --check --workspace -- --all-features` failed against it —
  the confirmation the finding admitted it never had. It named **one** file:
  `query-905f7d2f…` (`lfg::components::edit::EditRow::get`,
  `bot-modules/lfg/src/components/edit.rs:50-68`). `cargo sqlx prepare --workspace
  -- --all-features` rewrote exactly that one entry; `--check` then exited 0.
- **The drift was one inverted `nullable` array, and nothing else.** The committed
  entry held `[true,true,true,true,true,false]` for
  `p.owner_id, p.activity, p.start_time, p.description, p.fireteam_size,
  u.timezone` — i.e. the five `NOT NULL` base-table columns marked nullable and
  the `LEFT JOIN`ed `timezone` marked non-null, which is backwards on both sides.
  It is now `[false,false,false,false,false,true]`. Cache total is unchanged at
  **237** entries: **1 modified, 0 added, 0 deleted**.
- **Two corrections to this finding's own record, both honest downgrades:**
  - **The scope was smaller than CC-5 estimated.** CC-5 named three drifted
    entries; only `905f7d2` was actually drifted. `fc6caa8e` matches the schema
    today, and the "missing `895e6b8`" this finding cited as offline evidence is
    **not missing — it no longer exists**: a full regen added zero entries, so the
    query that once hashed to it was removed from the source by a later refactor.
    A hash absent from `.sqlx` is only drift if some live `query!` still needs it,
    which a regen is what proves. Do not read a missing hash as a missing entry.
  - **No runtime behaviour changed, and could not have.** Every column in this
    query carries an explicit `!`/`?` override, which is precisely what *overrides*
    the cached inference — `EditRow`'s field types are byte-identical before and
    after. This finding's value is entirely the one it claimed: it restores
    `prepare --check` as a gate that can detect a *future* real drift. It fixed no
    bug, and should not be recorded as having fixed one.
- **No regression test, because the gate is the test.** `prepare --check` is a
  fails-before / passes-after check against the schema itself (red at
  `6d51f6fc`, exit 0 after), which is strictly stronger than anything a `tests/`
  file could assert about a JSON cache. A test pinning the `nullable` array would
  only restate the regenerated file.
- **Gates (all run with `SQLX_OFFLINE=true`, so they also prove the regenerated
  cache builds the workspace):** `cargo +nightly clippy --workspace --all-targets
  -- -D warnings` exit 0 and `.bacon-locations` empty; `cargo test --workspace
  --no-fail-fast` **615 passed / 0 failed / 7 ignored** (unchanged from the
  palworld #4 baseline, as expected for a no-behaviour change);
  `cargo +nightly check -p dashboard --features ssr` exit 0; `cargo +nightly fmt`
  clean. No `Cargo.toml` change, so no `cargo machete`. No new
  `#[allow]`/`#[expect]` — no Rust source was touched at all.
- **Residual:** the regen must be re-run against an **empty** DB, never a
  populated one — the throwaway was created for this task and dropped after. Also
  note `.sqlx` now agrees with `migrations/` **as of `0023_verified_role`**; the
  gate only stays green if future SQL tasks regenerate rather than hand-edit.
- **Recorded 2026-08-05.** Not a new discovery: [CC-5](#cc-5-runtime-sqlxquery-bypassing-compile-time-macros)
  hit this on 2026-07-24, judged it out of its own scope ("pre-existing, not
  CC-5"), and wrote *"worth its own finding"* — which was never opened. Doing so
  now so it stops being a footnote inside a closed finding. (Numbered after CC-9,
  which sits in the Deep-sweep section below.)
- **Where:** `.sqlx/` at the workspace root, against `migrations/`.
- **What:** `cargo sqlx prepare --workspace -- --all-features --check` fails on a
  clean tree. CC-5 attributed it to missing/stale `LEFT JOIN` nullability
  entries and named three: gambling `895e6b8` / `fc6caa8e`, lfg_posts `905f7d2`.
- **Verification state (be honest about this):** *not* re-confirmed at recording
  time — `--check` needs a `DATABASE_URL` with the schema applied, and migrating
  a database is the human's step, not an agent's. What *was* checked offline
  supports the claim: of the three entries CC-5 named, `query-fc6caa8e…` and
  `query-905f7d2…` are present in `.sqlx/` but **no entry starts with
  `query-895e6b8`** — i.e. the missing entry is still missing. Note the cache has
  been touched since (`cd003007`, `374ec7e5`, `c7605e43`), so incremental
  per-task regens may have moved the picture; re-run `--check` first.
- **Why it matters:** The cache is what makes `SQLX_OFFLINE=true` builds
  trustworthy — CI's `prepare --check` is the gate that proves the committed SQL
  still matches the schema. While it is red for a *pre-existing* reason, that
  gate cannot tell anyone whether a *new* change broke something: every task
  since has had to eyeball its own `.sqlx` delta and revert unrelated drift by
  hand (the lfg #4 and CC-5 precedents both record doing exactly that).
- **Suggested fix:** Regenerate the whole cache in one pass —
  `cargo sqlx prepare --workspace -- --all-features` — against an **empty,
  freshly-migrated** database, never a populated one: `LEFT JOIN` nullability
  inference is plan- and statistics-sensitive, so a cache built against a
  populated DB bakes in different nullability than CI's empty one and the check
  fails again. Commit the whole `.sqlx/` diff as its own change so the noise is
  never mixed into a behavioural one. This is a **structural enabler** — it
  restores the gate every later SQL task depends on.

## Deep-sweep findings

_Deep sweep pass over the whole workspace, 2026-07-17. These are latent
defects that sit **underneath** CC-1…CC-8 — the concrete failure scenarios the
first-pass structural findings only hinted at. Per-module detail lives in the
`DS-#` entries of each module file; this section records the one genuinely
cross-cutting theme plus an index._

### CC-9. Read-modify-write on economy/counter rows with **absolute** overwrite (race class)  ·  #3  ·  high
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
  Umbrella — every enumerated site is now closed; the umbrella itself is the
  human's call to close. Each site was its own task.
  - **Closed:** [gambling DS-1…DS-5, DS-7](gambling.md),
    [gold-star DS-1](gold-star.md), [temp-voice DS-2](temp-voice.md),
    **[gambling DS-9](gambling.md) (`/shop buy`)**,
    **[gambling DS-10](gambling.md) (`/shop sell`)**,
    **[gambling DS-11](gambling.md) (`/dig`)**,
    **[gambling DS-12](gambling.md) (`/work` mine accrual)**,
    **[gambling DS-13](gambling.md) (`/craft`)**,
    **[gambling DS-14](gambling.md) (every wager game — `GameRow::save`)**,
    **[gambling DS-15](gambling.md) (`/prestige` gem award)**.
  - **`gambling` is now fully closed.** DS-15 removed the last absolute
    `EXCLUDED` write in the module; `common/shop/mod.rs`, `commands/craft.rs`,
    `models/game_row.rs` and `commands/prestige.rs` are all clear (DS-9/DS-10
    removed the shop helpers, DS-13 removed `CraftManager::save`, DS-14 removed
    `GameRow::save`, DS-15 split the prestige write by semantics).
  - **`levels` (the last enumerated site) is now closed** by
    **[levels DS-1](levels.md)** — the site re-enumerated here on 2026-07-29
    after `gambling` closed (`manager.rs:325` `FullLevelRow::save` and `:435`
    `GuildLevelRow::save`, both `xp = EXCLUDED.xp, total_xp = EXCLUDED.total_xp,
    level = EXCLUDED.level, message_count = EXCLUDED.message_count`). Both
    `save`s were removed in favour of `accrue_message`, which increments in SQL.
    The third pass had traced the *cooldown* clean; DS-1 shows it was not — it
    was an in-memory comparison against the **snapshot's** `last_xp`, so the
    same interleave that lost the XP also let both handlers spend one cooldown
    window. It is now the write's own `WHERE last_xp <= now() - interval
    '1 minute'` guard.
  - **DS-1 adds a third corrective shape.** DS-11 gave the *accrual*
    compare-and-swap and DS-15 the *split-by-semantics* rule; DS-1 covers a
    **derived column** — `level` is a function of `xp`, so it cannot be an
    increment. The pattern is: increment the accumulators in one guarded
    statement, `RETURNING` the post-increment values, then apply the derived
    change in a second statement guarded on the value you saw
    (`WHERE level = $old AND xp >= $threshold`). The loser of the race matches
    no row rather than writing back a stale derivation.
  - **Re-swept 2026-07-29 (levels DS-1 task):** every remaining `EXCLUDED.` in
    the workspace was re-read. What is left is settings/config rows
    (`zayden-app/src/config/tables/*`, `marathon/announce.rs`), catalog and
    metadata upserts (`destiny2/db/compendium.rs`, `palworld/{link,upload}.rs`,
    `family/manager.rs` username), the deliberate CAS arms from DS-11/DS-12
    (`CASE WHEN … THEN EXCLUDED.mine_activity`), and the monotone
    `GREATEST(stats.col, EXCLUDED.col)` score upsert
    (`gambling/sql/StatsManager/higherlower.sql`) — all last-writer-wins or
    race-safe **by design**, none of them read-modify-write economy/counter
    rows. No unenumerated site was found.
  - **DS-15 sharpened the corrective pattern a third time.** DS-11 established
    that a *time-based accrual* needs a compare-and-swap rather than an
    increment. DS-15 adds: not every column in an absolute write is a defect.
    `/prestige` writes `coins`, `gems` and `stamina` together, but only `gems` is
    a read-modify-write — `coins` and `stamina` are deliberate **resets**, whose
    whole purpose is to ignore the pre-image (put the player back to
    `START_AMOUNT` with a full stamina bar so they can keep playing immediately).
    Converting those to increments would be a *bug*. Split the write by
    semantics — increment what accumulates, overwrite what resets — rather than
    mechanically converting every `EXCLUDED.col` found by grep.
  - **New sub-class surfaced by DS-11:** where the value being persisted is a
    *time-based accrual* rather than a per-action reward, the corrective pattern
    is not an atomic increment — that pays the same accrued window twice. It is a
    compare-and-swap on the accrual's watermark (`mine_activity`), with the
    credit conditional on winning the swap. See [gambling DS-11](gambling.md) for
    the shape. **This sub-class is now closed:** DS-12 gave `/work` the same
    treatment, so the two collectors of the `gambling_mine` accrual interlock —
    the loser of the swap is paid `0` for the window while still earning its own
    base pay. `MinePayout` lives in `models/mod.rs` beside the `MineAmount` trait
    and is the shared carrier for any future collector.

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
  **Corrected by the 2026-07-29 pass:** "mostly clean" understated it. The
  `check_and_set` gate covers only same-user *game* replays, and the games'
  *settlement* — not the `bet.sql` decrement — went through `GameRow::save`, an
  absolute whole-row write, so every concurrent `/daily`/`/work`/`/dig`/`/send`
  credit was clobbered. See [gambling DS-14](gambling.md), now closed.

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
