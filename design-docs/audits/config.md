# Audit: config (zayden-app)

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Hosts the M1 settings backbone: the `SettingsRow`/`SettingsStore` pattern with a
`SettingsRegistry` wiring the per-feature stores (`channels_settings`,
`lfg_settings`, `music`, `roles_settings`, `suggestions_settings`,
`support_settings`, `temp_voice_settings`, `ticket`) plus cache invalidation.
That layer is clean and is the workspace's settings convention. The residual
debt is in the newer **entitlement** subsystem.

## Findings

### 3. `#[expect]` in entitlement service  ·  #7  ·  low
- **Status:** `unclear`
- **Where:** `src/entitlement/service.rs:78`.
- **What / Why / Fix:** One CC-3 escape-hatch; triage per
  [CC-3](_cross-cutting.md#cc-3).

### 4. `SettingsRegistry` is the shared bot/web backend — the key CC-8 enabler  ·  #8  ·  info
- **Status:** `unclear`
- **Where:** `src/config/registry.rs`, `src/config/tables/*`,
  `src/state/app_state.rs`.
- **What:** Both the bot and the dashboard write settings through this one
  registry, which is exactly why the CC-8 de-duplication is low-risk: pointing a
  `setup` command and a web form at the same store is already the design.
- **Why it matters:** Not a defect — it's the reason moving config to the web
  doesn't require new plumbing. New dashboard config pages (music/ticket/
  suggestions/reaction-roles) should reuse the existing `SettingsRow` stores
  rather than add server-side SQL.
- **Suggested fix:** None here; noted so the CC-8 follow-ups reuse this backend.
  See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: `SettingsRow`/`SettingsStore`/`SettingsRegistry` is the clean,
  intended pattern; `config/tables/*` each implement `SettingsRow` uniformly;
  `state/app_state.rs` composes them.
- #1 DB access (settings): compile-time `query_as!` throughout `config/tables/*`.
- #3 Async: `std::fs::read_to_string` in `config/bot_config.rs:228` is a
  startup-only config load (acceptable, not on a request path); no locks across
  `.await`.
- #4 Stringly typing: entitlement provider/tier handling is typed; Ko-fi/Discord
  providers sit behind an enum-dispatched provider trait.
- #6 Tests: `tests/` files present (entitlement, settings).
