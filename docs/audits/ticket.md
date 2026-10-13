# Audit: ticket

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

The M1 reference migration: the DB-generic `TicketGuildManager<Db>` /
`TicketManager<Db>` traits were removed and the sqlx code moved concrete
(`TicketRow`/`TicketGuildRow` inherent `PgPool` methods with `query!`/
`query_as!`). Architecture is clean and it is **not** subject to CC-1 — it is
the template the other manager crates should follow. The one real gap is tests:
the ticket lifecycle's open/close/remove state transitions still have no
regression coverage.

## Findings

### 1. No integration tests  ·  #6  ·  med
- **Status:** `open`
- **Where:** crate has no `tests/` directory (`ticket_manager.rs`,
  `support_guild_manager.rs`, `slash_commands/ticket/{open,close,remove,create}.rs`).
- **What:** The ticket lifecycle (open → fixed → close → remove, support-guild
  routing) has no regression coverage.
- **Why it matters:** State-machine logic with side effects is exactly what a
  test net protects; a future refactor has nothing to catch it.
- **Suggested fix:** Add `tests/` for the pure transition logic and the
  support-guild resolution; DB paths once a test-pool harness exists. See
  [CC-6](_cross-cutting.md#cc-6).

## Clean
- #1 Architecture: concrete `PgPool`, no generic-DB trait (the CC-1 exemplar);
  clean `slash_commands/{ticket,support}` tree; `components.rs`/`modal.rs`
  interaction routing.
- #1 DB access: compile-time `query!`/`query_as!` only; no ad-hoc SQL.
- #2 Dead code: none found; M1 dropped the `GuildTable`/`TicketTable` type-param
  threading.
- #3 Async: no blocking I/O; no locks across `.await`.
- #4 Stringly typing: no raw domain-string matching.
