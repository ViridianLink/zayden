# Audit: temp-voice

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Recently extended with the M4 button control-panel; structure is clean
(`actions/` shared-mutation layer, `components/` one-file-per-button-group,
`commands/`, `events/`). Concrete `PgPool` throughout, no inline `#[cfg(test)]`,
and five integration test files including `actions_authz.rs`.

## Findings

### 3. Region list hardcoded, flagged for API sync  ·  #4 / #5  ·  low
- **Status:** `unclear`
- **Where:** `src/components/mod.rs:43` — `// TODO: Can regions be pulled from
  Discord API to avoid future drift`.
- **What:** The voice-region option set is a hardcoded constant list that can
  drift from Discord's actual regions.
- **Why it matters:** Silent staleness if Discord adds/renames a region.
- **Suggested fix:** Either resolve regions from the Discord API at startup and
  cache, or leave a dated note that manual sync is accepted. Low priority.

## Clean
- #1 Architecture: `actions`/`components`/`commands`/`events` split is clean and
  mirrors LFG conventions; `ModuleComponent`/`ModuleModal` wired in `bot/`.
- #2 Dead code: M4 dropped the stubbed `waiting`/`info` arms; no soft stubs found.
- #3 Async: no blocking I/O; no locks across `.await`.
- #2 (bugs) The 4a extraction fixed the inverted `delete` owner check and the
  `password` option-key mismatch — verified resolved.
