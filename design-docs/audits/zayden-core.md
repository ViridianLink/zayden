# Audit: zayden-core

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

The shared foundation crate: `Ctx`, `Module`/`ModuleComponent`/`ModuleModal`
traits, cron scaffolding, cache, scope/snowflake/format helpers, templates. It is
where the generic `<Db: Database>` trait bounds that propagate into the manager
crates originate (`cron.rs`, `events.rs`, `module.rs`), so it is both a CC-1
*source* and the place a de-generalisation would start. One inline test module.

## Findings

### 1. Generic `<Db: Database>` trait bounds in core traits  ·  #1  ·  med
- **Where:** `src/cron.rs`, `src/events.rs`, `src/module.rs`.
- **What:** The core `Module`/cron/event traits are generic over the sqlx
  `Database`, which is what forces every downstream manager to be generic too
  (the root of CC-1).
- **Why it matters:** As long as core stays generic over `Db`, the manager
  crates can't cleanly go concrete. This is the *keystone* of the CC-1 migration
  — de-generalising here (to `Postgres`) unblocks all the module-level cleanups.
- **Suggested fix:** Plan the CC-1 migration top-down: pin the core traits to
  `Postgres` first, then convert managers crate-by-crate. See
  [CC-1](_cross-cutting.md#cc-1).

### 2. Inline `#[cfg(test)]` module  ·  #6  ·  low
- **Where:** `src/snowflake.rs:13`.
- **What / Why / Fix:** See [CC-2](_cross-cutting.md#cc-2). Move to `tests/`.

## Clean
- #2 Dead code: none found.
- #3 Async: cache/cron helpers non-blocking; no locks across `.await` observed.
- #4 Stringly typing: `format_num`/`scope`/`templates` are typed helpers.
- #7 Lint: no `#[expect]`/`#[allow]`.
