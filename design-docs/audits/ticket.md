# Audit: ticket

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

The M1 reference migration: the DB-generic `TicketGuildManager<Db>` /
`TicketManager<Db>` traits were removed and the sqlx code moved concrete
(`TicketRow`/`TicketGuildRow` inherent `PgPool` methods with `query!`/
`query_as!`). Architecture is clean and it is **not** subject to CC-1 — it is
the template the other manager crates should follow. The one real gap is tests:
the crate ships **zero** `tests/` files despite non-trivial open/close/remove
state transitions.

## Findings

### 1. No integration tests  ·  #6  ·  med
- **Where:** crate has no `tests/` directory (`ticket_manager.rs`,
  `support_guild_manager.rs`, `slash_commands/ticket/{open,close,remove,create}.rs`).
- **What:** The ticket lifecycle (open → fixed → close → remove, support-guild
  routing) has no regression coverage.
- **Why it matters:** State-machine logic with side effects is exactly what a
  test net protects; a future refactor has nothing to catch it.
- **Suggested fix:** Add `tests/` for the pure transition logic and the
  support-guild resolution; DB paths once a test-pool harness exists. See
  [CC-6](_cross-cutting.md#cc-6).

### 2. Support-guild / panel config belongs on the dashboard  ·  #8  ·  low
- **Where:** `src/support_guild_manager.rs` + the config-shaped
  `slash_commands/support/*`.
- **What:** Support-guild routing config is admin setup with no dashboard
  equivalent yet.
- **Why it matters:** Config/admin is the dashboard's domain (see the destiny2
  loadout precedent).
- **Suggested fix:** Add a ticket/support section to the settings page against the
  concrete `PgPool` manager; keep open/close/claim ticket interactions in-bot.
  See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: concrete `PgPool`, no generic-DB trait (the CC-1 exemplar);
  clean `slash_commands/{ticket,support}` tree; `components.rs`/`modal.rs`
  interaction routing.
- #1 DB access: compile-time `query!`/`query_as!` only; no ad-hoc SQL.
- #2 Dead code: none found; M1 dropped the `GuildTable`/`TicketTable` type-param
  threading.
- #3 Async: no blocking I/O; no locks across `.await`.
- #4 Stringly typing: no raw domain-string matching.

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lens: Discord-API correctness (component limits)._

### DS-1. `/support list` builds a select menu from **every** FAQ message → breaks past 25 options  ·  Pass 3  ·  med
- **Status:** `complete — 82f308a2`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-19):** `.take(25)` the filtered option stream (also stops the
  full-channel pagination early) and short-circuit with a friendly notice when the
  option set is empty (a 0-option menu is likewise rejected). The `enumerate`
  index used as the option `value` is unchanged, so the follow-up handler's
  index→message mapping still holds. The value-stability fragility (Secondary)
  is left as-is per CC-8's move-to-dashboard direction.
- **Where:** `bot-modules/ticket/src/slash_commands/support/list.rs:29-51`.
- **What:** `menu_options` is built by paginating the whole FAQ channel
  (`faq_channel_id.widen().messages_iter(http)`) and mapping **every** message to a
  `CreateSelectMenuOption`, with **no `.take(25)`**. Discord string select menus
  accept at most 25 options; the resulting `edit_response` with 26+ options is
  rejected with a 400.
- **Failure scenario:** A support guild's FAQ channel accumulates 26+ messages
  (each FAQ entry is one message). A user runs `/support list` → the builder emits
  26+ options → `edit_response` returns HTTP 400 (invalid select menu) → the
  command errors and the FAQ picker is unusable. The feature silently works while
  the channel is small and breaks permanently once it crosses 25 entries — exactly
  when a FAQ is mature enough to need a picker.
- **Secondary:** `messages_iter` paginates the entire channel on every invocation
  (unbounded read), and the option `value` is the running `enumerate()` index, so
  the selected index only maps back correctly if the pagination order is stable
  between the list and the follow-up interaction.
- **Why it matters:** A support-facing command breaks at scale with no guard rail;
  the 25-option ceiling is a hard Discord limit, not a soft one.
- **Confidence:** confirmed (no bound on the iterator; 25-option cap is fixed).
- **Suggested fix:** `.take(25)` the option stream (and ideally paginate the FAQ
  picker, or move the FAQ list to the dashboard per CC-8). Guard against an empty
  option set too (a 0-option select menu is also rejected).

### DS-2. `/lfg tags` (add/remove) can emit an empty select menu → 400  ·  Pass 3  ·  low
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `bot-modules/lfg/src/commands/tags.rs:60-83` (add) and `:91-113`
  (remove).
- **What:** `options` is filtered from `forum_channel.available_tags`; when the
  filter leaves nothing (add: all tags already applied; remove: no tags applied;
  or a forum with no tags configured), `options` is empty and `max_values` is 0.
  Discord rejects a string select menu with 0 options (min 1) and `max_values(0)`.
- **Failure scenario:** A thread already carries every available forum tag → user
  runs `/lfg tags add` → empty menu → `edit_response` 400 → command errors instead
  of a friendly "no tags to add" message. (Option count itself is safe: forum tags
  cap at 20 < 25.)
- **Why it matters:** Minor UX break at a natural boundary (fully-tagged thread).
- **Confidence:** confirmed (logic traced; empty-menu rejection is a fixed rule).
- **Suggested fix:** If `options.is_empty()`, reply with an ephemeral notice
  instead of building the menu.
