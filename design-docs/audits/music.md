# Audit: music

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

One of the healthiest crates. Async concurrency is handled deliberately
(`tokio::sync::Mutex` for the per-guild `GuildPlayer`, `DashMap` for the players
map), settings are read through the M1 `SettingsRegistry` (`cx.app.settings.music`)
rather than ad-hoc, and it has the second-best test coverage in the workspace
(7 integration files: embeds, permissions, player, queue, resolve, spotify,
youtube). No CC-1 (in-memory manager, not DB-generic).

## Findings

_None outstanding._

## Clean
- #1 Architecture: clear `commands/` · `components/` · `resolve/` · manager /
  player / queue / voice split; settings via `SettingsRegistry`.
- #1 DB access: n/a — playback state is in-memory by design; no ad-hoc SQL.
- #3 Async: **correct** — `tokio::sync::Mutex` held across `.await` is
  intentional and safe; `DashMap` entries not held across `.await`.
- #4 Stringly typing: control-panel routing is namespaced; no raw domain strings.
- #6 Tests: 7 integration files covering real behaviour (queue ops, permission
  gating, resolver parsing).
