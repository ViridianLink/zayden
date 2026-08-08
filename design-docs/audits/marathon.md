# Audit: marathon

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Best-tested crate in the workspace (13 integration files + a committed
`tests/fixtures/` corpus) and concrete `PgPool` throughout. The cross-source
merge/precedence design is well isolated (`merge.rs`, per-transport parsers under
`transport/`). No CC-1, no runtime-SQL, no inline tests. Essentially clean; only
housekeeping notes.

## Findings

### 1. Fixture regeneration must stay documented  ·  #6  ·  low
- **Status:** `unclear`
- **Where:** `tests/fixtures/*.json`, transport parsers in `src/transport/`,
  `src/news.rs`.
- **What:** Fixtures are captured from live endpoints + FlareSolverr; there is a
  known `.gitignore *.json` gotcha (fixtures must be force-added) per project
  memory.
- **Why it matters:** If regeneration drifts from the documented procedure, a
  contributor can silently commit stale or zero fixtures.
- **Suggested fix:** Ensure the capture procedure is recorded in the crate
  (a `tests/fixtures/README` or module doc-comment) so it survives without the
  chat context.

## Clean
- #1 Architecture: `transport/` per-source parsers + `merge.rs` consensus layer;
  concrete `PgPool`.
- #1 DB access: compile-time macros; the `.query(&[...])` calls are HTTP query
  params (reqwest), not SQL.
- #2 Dead code: none found.
- #3 Async: network parsers async; no blocking on hot paths.
- #4 Stringly typing: `WeaponStat` enum + `FromStr` landed in M2 with tests.
- #6 Tests: comprehensive (cron, embeds, html, merge, per-source parsers).
