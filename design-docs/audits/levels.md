# Audit: levels

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Small (~715 LOC) and functional: concrete `PgPool` manager, a typed
`LevelsCustomId` component enum, and integration tests under `tests/`.

## Findings

_None outstanding._

## Clean
- #1 DB access: concrete impl uses compile-time `query!`/`query_as!`/
  `query_scalar!` (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: message-create XP path is non-blocking; no locks across `.await`.
