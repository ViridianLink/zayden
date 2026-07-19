# Audit: config (zayden-app)

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Hosts the M1 settings backbone: the `SettingsRow`/`SettingsStore` pattern with a
`SettingsRegistry` wiring the per-feature stores (`channels_settings`,
`lfg_settings`, `music`, `roles_settings`, `suggestions_settings`,
`support_settings`, `temp_voice_settings`, `ticket`) plus cache invalidation.
That layer is clean and is the workspace's settings convention. The debt is in
the newer **entitlement** subsystem, which uses runtime `sqlx::query(...)`
instead of the compile-time macros the settings tables use, and has an inline
test module.

## Findings

### 1. Runtime `sqlx::query(...)` in entitlement service  ·  #1  ·  med
- **Where:** `src/entitlement/service.rs:111`, `:309` (hand-written
  `DELETE`/query with `.bind(...)`).
- **What / Why / Fix:** See [CC-5](_cross-cutting.md#cc-5). Convert to
  `query!`/`query_as!` and regenerate `.sqlx/`. Notable because the same crate's
  `config/tables/*.rs` all use compile-time macros — the entitlement code is
  inconsistent with its own crate.

### 2. Inline `#[cfg(test)]` module  ·  #6  ·  med
- **Where:** `src/entitlement/types.rs:144`.
- **What / Why / Fix:** See [CC-2](_cross-cutting.md#cc-2). Move to `tests/`
  (the crate already has one `tests/` file, so the harness exists).

### 3. `#[expect]` in entitlement service  ·  #7  ·  low
- **Where:** `src/entitlement/service.rs:78`.
- **What / Why / Fix:** One CC-3 escape-hatch; triage per
  [CC-3](_cross-cutting.md#cc-3).

### 4. `SettingsRegistry` is the shared bot/web backend — the key CC-8 enabler  ·  #8  ·  info
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

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lens: cache-invalidation / state integrity._

### DS-1. `grant` writes the granted tier to the cache row, not the scope's aggregate max → silent downgrade  ·  Pass 7  ·  med
- **Status:** `complete — 0150d39d`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Where:** `src/entitlement/service.rs:100` (`grant` →
  `refresh_cache_row(&scope, tier)`) vs. `:124`,`:149`,`:331` (`revoke` /
  expiry sweep → `refresh_cache_row_from_db`, which recomputes the aggregate
  `MAX(tier)`).
- **What:** A scope's effective tier is the **max across all active
  `entitlements` rows** for that scope (see `aggregate_tier_from_db`, and `allows`
  taking `u.max(g)`). `grant` bypasses that aggregation: it writes the single
  granted `tier` straight into the denormalised `entitlement_cache` row. Since
  `load_tier_from_db` trusts the `entitlement_cache` row before it ever aggregates
  (`:248-264`), a `grant` of a **lower** tier stamps the cache below the true max.
- **Failure scenario:** a guild holds an active `ultra` entitlement
  (`provider=discord, external_id=A`). A second, lower grant arrives —
  `grant(Guild, Pro, provider=kofi, external_id=B)`. The `entitlements` table now
  has both rows (aggregate max = ultra), but `grant` calls
  `refresh_cache_row(scope, Pro)`, so `entitlement_cache.tier = 'pro'`. Every
  `allows(.., Ultra)` check for that guild now returns **false** until a revoke or
  the expiry sweep happens to recompute this scope — the paid Ultra features go
  dark even though the Ultra entitlement is still active.
- **Suggested fix:** make `grant` call `refresh_cache_row_from_db(&scope)` (the
  aggregate path) after the upsert, exactly like `revoke`; drop the tier-argument
  form of `refresh_cache_row` or reserve it for the recompute helper.
  **Confidence: confirmed** on the logic asymmetry; **plausible** on frequency
  (requires ≥2 active entitlements of different tiers on one scope).

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
- #6 Tests: one `tests/` file present (entitlement) — extend to cover the
  service DELETE/revoke paths once finding #1 lands.
