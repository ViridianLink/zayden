# Audit: palworld

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Solid, recently-built crate: concrete `PgPool`, strong test coverage (12
integration files), and — importantly — its blocking save-file I/O is correctly
offloaded via `tokio::task::spawn_blocking` (`client.rs`, `commands/upload.rs`).
The type modelling (`model.rs` element enum with alias-tolerant `parse`) is a
good example of #4 done right. Minor: one inline test module and one blocking
`std::fs` call worth confirming is off the async path.

## Findings

### 1. Inline `#[cfg(test)]` module  ·  #6  ·  med
- **Where:** `src/commands/breed_plan.rs:147`.
- **What / Why / Fix:** See [CC-2](_cross-cutting.md#cc-2). Move to `tests/`
  (the crate already has 12 integration files — harness is established).

### 2. Confirm `cron.rs` `std::fs::remove_file` is not on the async reactor  ·  #3  ·  low
- **Where:** `src/cron.rs:16` (`std::fs::remove_file`), and the sync `std::fs`
  helpers in `src/save/mod.rs`.
- **What:** Most save I/O is wrapped in `spawn_blocking`; verify the cron
  cleanup `remove_file` (a single unlink) either runs inside a blocking context
  or is cheap enough to accept. `save/mod.rs` helpers appear to be called only
  from within `spawn_blocking` closures.
- **Why it matters:** A stray sync `remove_file` on the async reactor is a
  (small) stall; a single unlink is usually tolerable but worth a glance.
- **Suggested fix:** Confirm the call site; wrap if it runs on an async task.

### 3. Breed-plan / Paldex displays are better as dashboard read-views  ·  #8  ·  low
- **Where:** `src/commands/breed_plan.rs`, `src/commands/breed_for.rs`, Paldex
  display paths.
- **What:** Breeding-path and Paldex output is data-dense and better browsed on a
  web page than paged in an embed.
- **Why it matters:** UX gain; the breeding data is already computed from
  DB/model.
- **Suggested fix:** Add dashboard breed-plan/Paldex views; keep save-upload and
  live server ops in-bot. See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: `transport/` (fandom/pelican) + `save/` + `commands/` +
  `client.rs` cleanly separated; concrete `PgPool`.
- #1 DB access: compile-time macros; `.query(&[...])` are HTTP params, not SQL.
- #3 Async: **correct** — save decode/load offloaded via `spawn_blocking`.
- #4 Stringly typing: `model.rs` element enum has an alias-tolerant `parse`
  (handles source typos like `"electricty"`) — good.
- #6 Tests: 12 integration files (breeding, upload, save decode/world, guild).
