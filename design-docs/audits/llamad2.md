# Audit: llamad2

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

A grab-bag of server-specific novelty handlers (~588 LOC): hello, goodmorning,
socials, dungeon/raid reports, counting-fail and "goof" counters. The notable
issues are two persistent counters stored as **flat JSON files written with
blocking `std::fs` on the async message path** — both a #3 (async) and #5 (data
placement) problem.

## Findings

### 1. Blocking `std::fs` counter persistence on async path  ·  #3  ·  med
- **Where:** `src/counting_fail.rs:31-48` (`OpenOptions`… `write_all`, file
  `countingFails.json`), `src/goof.rs:26-43` (file `dumbCount.json`). Both inside
  `async fn run(...)`.
- **What:** Each invocation opens, reads, and rewrites a JSON file synchronously
  on the async reactor.
- **Why it matters:** Blocking file I/O on an async task stalls the runtime
  thread; and a flat file in the process CWD is fragile (lost on redeploy, not
  shared across instances, races between concurrent messages).
- **Suggested fix:** Move both counters to a DB table (a single `counters` row
  per kind), read/write via `query!`. Removes the blocking I/O and the data-
  placement problem at once. See also [CC-5](_cross-cutting.md#cc-5).

### 2. Counter state belongs in the DB  ·  #5  ·  med
- **Where:** same as #1 (`countingFails.json`, `dumbCount.json`).
- **What / Why / Fix:** Persistent per-guild counters stored outside the DB.
  Fold into the fix for #1.

### 3. No integration tests  ·  #6  ·  low
- **Where:** no `tests/` directory. Low priority — mostly cosmetic handlers.

## Clean
- #1 Architecture: one file per handler; simple.
- #4 Stringly typing: handler dispatch is in `bot/` bindings; nothing egregious.
- #7 Lint: no `#[expect]`/`#[allow]` in this crate.
