# Audit: suggestions

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Small (~600 LOC), clean `slash_command` + `components` + `modal` + `reaction` +
`guild_manager` layout. Carries the DB-generic `async_trait` pattern (CC-1) and
no `tests/`. Otherwise clean.

## Findings

### 1. DB-generic `async_trait` manager  ·  #1  ·  med
- **Status:** `complete — b4bb8582`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-23):** CC-1 concrete-`PgPool` migration (fourth module after the
  `gold-star`/`levels`/`reaction-roles` pilots). Dropped the `#[async_trait] trait
  SuggestionsGuildManager<Db: Database>` and its lone `impl … for GuildTable`
  binding. The single `get` query is now a concrete `PgPool` associated function
  `SuggestionsGuildRow::get` with the `query_as!` body living in the crate
  (`guild_manager.rs` renamed to `manager.rs`, matching the pilots).
  `FetchSuggestions::run` and `Suggestions::reaction` lost their `<Db, Manager>`
  generics and now take `&PgPool` directly; the three call sites
  (`bot/src/bindings/suggestions/slash_command.rs` and the two
  `bot/src/handler/reaction_{add,remove}.rs`) drop their turbofish and the
  `GuildTable`/`Postgres` imports. `bot/src/bindings/suggestions` keeps only its
  `ModuleCommand`/component/modal shims. **Behaviour-preserving:** the `get` query
  string was moved byte-identically, so the `.sqlx` cache is reused unchanged
  (`git status .sqlx` clean — no regeneration). Removed the now-unused
  `async-trait` dependency (`cargo machete` clean; the crate has no other
  `async_trait` use). Added `tests/manager.rs` pinning the `SuggestionsGuildRow`
  snowflake accessors (the row type the migration moved); `review_action` was
  already covered by `tests/review_threshold.rs` (DS-1). Only the larger generic
  managers — `gambling`, `family`, `lfg`, `temp-voice`, plus the `zayden-core`
  traits — now remain on CC-1.
- **Where:** `src/guild_manager.rs`, `src/slash_command.rs`, `src/reaction.rs`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1).

### 2. No integration tests  ·  #6  ·  low
- **Where:** no `tests/` directory.
- **What / Why / Fix:** Add coverage for the suggestion up/down tally logic. See
  [CC-6](_cross-cutting.md#cc-6).

### 3. Channel/threshold config belongs on the dashboard  ·  #8  ·  low
- **Status:** `complete — 6608720d`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Scope correction (2026-07-27):** re-tracing the finding split it in half. The
  **channel** half was already satisfied — `suggestions_channel_id` *and*
  `review_channel_id` are both written by `save_support_settings`
  (`dashboard/src/server/guild.rs`) and rendered in the settings page's Support
  section, and applying the ticket #2 check (does a bot-side editor exist at all?)
  found that it does not: `slash_command.rs` is `/fetch_suggestions`, a *read*.
  So there was no duplication and nothing stranded. The **threshold** half was the
  live defect, and it is a data-placement one rather than a placement preference.
- **Fix (2026-07-27):** Made the promote/demote bounds per-guild config with the
  module crate owning the rule, per CC-8's "one owner for the normalisation
  contract".
  - **The defect.** `review_action` (`reaction.rs`) hard-coded `delta >= 20` /
    `delta <= 15` as `const` literals, with no column and no dashboard field. A
    guild whose suggestions never reach +20 net upvotes had a **permanently empty
    review channel** — the whole review flow inert — and a very large guild had it
    flooded; neither could change the bar short of a code change and redeploy.
  - **Schema** (`0020_suggestions_thresholds`): `promote_threshold` /
    `demote_threshold` `integer NOT NULL`, defaulting to `20`/`15` so every
    existing guild is behaviourally unchanged, plus a
    `CHECK (demote_threshold < promote_threshold)` — the hysteresis gap only
    exists while demote < promote; inverted, every reaction satisfies *both*
    branches and a review post flaps between created and deleted.
  - **The module owns the rule.** New `suggestions::ReviewThresholds` holds the
    defaults, the `new` constructor that enforces the invariant (the demote side
    yields to `promote - 1`, `saturating_sub` so `i32::MIN` doesn't wrap), and a
    `parse` for the web form's strings. `review_action` now takes it as a
    parameter; `Suggestions::reaction` reads it from `SuggestionsGuildRow`, whose
    `SELECT` gained both columns. The audit-log reason on the demote delete no
    longer says a hard-coded "15".
  - **Dashboard editor.** Both fields on the existing Support form
    (`save_support_settings` gained the two params and calls
    `ReviewThresholds::parse`, so the web editor cannot write a pair the bot would
    reject), with a `page-lead` explaining the hysteresis. `SettingField` gained an
    optional `pattern` prop — its digits-only default would have blocked a negative
    demote threshold, which is a legitimate "only remove once net-negative"
    setting. `dashboard` gains an ssr-gated `suggestions` dep, mirroring how it
    already depends on `ticket`/`reaction-roles`.
  - **Regression test** `tests/review_threshold.rs` (extended, 12 tests): the
    fails-before case is `small_guild_can_lower_the_bar_it_could_never_reach` —
    it asserts a +5-net suggestion is `Promote` at `(4, -2)` while the same input
    is `Demote` under the defaults, and could not even be *expressed* before, since
    `review_action` took no thresholds. Also pins that the defaults equal the old
    literals, the normalisation/overflow edges, and the `parse` fallbacks.
    `tests/manager.rs` pins that the row's columns reach `review_action` unswapped.
  - **`.sqlx`:** regenerated with `cargo sqlx prepare --workspace -- --all-features`
    against a throwaway **empty, freshly-migrated** Postgres 18 — a clean 3-for-3
    swap (the three `suggestions_settings` queries). Following the CC-5 / lfg #4
    precedent, unrelated pre-existing drift the full regen surfaced was reverted so
    the diff stays scoped (see **Residual**).
  - **Gates:** `cargo +nightly clippy --workspace --all-targets -D warnings` clean,
    `cargo test` 316 passed / 0 failed, `-p dashboard --features ssr` clean, the
    wasm/hydrate check clean, `cargo +nightly fmt --check` clean. No new
    `#[allow]`/`#[expect]`.
- **Residual (pre-existing, not this finding):**
  1. The committed `.sqlx` has **zero** entries for the dashboard's eight
     `web_sessions` queries (and its `kofi_links` ones), so `SQLX_OFFLINE=true`
     builds of `dashboard --features ssr` already fail on clean `main`. The
     full regen produces them; they were reverted to keep this diff scoped. Same
     family as CC-5's recorded residual (the `905f7d2` `lfg_posts` LEFT-JOIN
     nullability entry, likewise reverted here) — together these two warrant the
     single "regenerate the whole cache against an empty DB" finding CC-5 called
     for.
  2. `cargo machete` reports `levels -- tokio` unused, introduced by `04a8ab2b`;
     untouched here.
  3. DS-1's secondary note still stands: `Demote` fires for every suggestion at or
     below the threshold, including brand-new ones at delta 0, so the unbounded
     `messages_iter` review-channel scan still runs on most reactions. Tunable
     thresholds change *where* that boundary sits but not the scan itself.
- **Where:** `src/guild_manager.rs` + the config-shaped `slash_command.rs`.
- **What:** Suggestion channel/threshold config (the `suggestions_channel` part is
  already surfaced under the dashboard's Support section; the rest is not).
- **Why it matters:** Config is the dashboard's domain; finishing it removes a
  bot-only editor.
- **Suggested fix:** Surface the full suggestions config on the settings page;
  keep the submit modal + vote reactions in-bot. See
  [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: clean component/modal/reaction/manager split.
- #1 DB access: concrete impl uses compile-time macros (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lenses: numeric/boundary, duplication/drift, Discord rate-limit._

### DS-1. Flipped subtraction in the demote threshold → downvoted suggestions never leave the review channel (+ per-reaction full-channel scan)  ·  Pass 9+5+3  ·  med
- **Status:** `complete — ba5bbf74`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/suggestions/src/reaction.rs:83,124`.
- **What:** Promotion uses the delta `pos_count - neg_count`:
  `if (pos_count - neg_count) >= 20 { …create/update review post… }`. The demote
  branch is written `else if (neg_count - pos_count) <= 15 { …delete review post… }`
  — the subtraction is **flipped**. Let `d = pos - neg`. The demote condition is
  `neg - pos <= 15` ⟺ `d >= -15`, but the comment on line 129 ("Positive delta
  fell below 15") and the promote branch make the intent unambiguous:
  `d <= 15` (i.e. `(pos_count - neg_count) <= 15`).
- **Failure scenario:** A suggestion is upvoted to `d = +25` → a review-channel
  post is created. It is then downvoted to `d = -20` (`neg - pos = 20`). Intended:
  `d = -20 <= 15` → delete the review post. Actual: `neg - pos = 20 <= 15` is
  **false** → the delete branch never runs, so the review post for a
  now-heavily-downvoted suggestion stays up permanently. Symmetrically, in the
  intended hysteresis gap `d ∈ [16,19]` the buggy condition (`d >= -15`) is *true*,
  so a reaction there spuriously tries to delete a post that should persist.
- **Secondary (Pass 3):** because `d >= -15` is true for almost every suggestion
  that hasn't hit +20, nearly every 👍/👎 reaction now enters the `else if` and runs
  `review_channel_id.messages_iter(http)` — an **unbounded pagination over the
  entire review-channel history** on every reaction event, a real rate-limit /
  latency hazard that the correct `d <= 15` bound would also largely avoid.
- **Why it matters:** The review queue accumulates stale posts for suggestions the
  community has since rejected, and each reaction hammers the Discord API scanning
  the review channel.
- **Confidence:** confirmed (arithmetic traced; comment confirms intent).
- **Suggested fix:** Change line 124 to `else if (pos_count - neg_count) <= 15`
  (keep the `>= 20` / `<= 15` hysteresis). Consider gating the review-channel scan
  behind a cheaper "is this message already tracked" check to avoid the full
  pagination on every reaction.
