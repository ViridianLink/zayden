# Audit: suggestions

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Small (~600 LOC), clean `slash_command` + `components` + `modal` + `reaction` +
`guild_manager` layout. Carries the DB-generic `async_trait` pattern (CC-1) and
no `tests/`. Otherwise clean.

## Findings

### 1. DB-generic `async_trait` manager  ·  #1  ·  med
- **Where:** `src/guild_manager.rs`, `src/slash_command.rs`, `src/reaction.rs`.
- **What / Why / Fix:** See [CC-1](_cross-cutting.md#cc-1).

### 2. No integration tests  ·  #6  ·  low
- **Where:** no `tests/` directory.
- **What / Why / Fix:** Add coverage for the suggestion up/down tally logic. See
  [CC-6](_cross-cutting.md#cc-6).

### 3. Channel/threshold config belongs on the dashboard  ·  #8  ·  low
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
