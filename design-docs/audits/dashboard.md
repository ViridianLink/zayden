# Audit: dashboard

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Leptos full-stack crate (SSR + WASM hydrate) with a clean `ui/pages` +
`ui/components` + `server/` + `web/` + `middleware/` split. Rendering-mode code is
correctly `#[cfg(feature = "...")]`-gated so the default clippy pass stays clean.
Two issues: a runtime `sqlx::query(...)` in the auth middleware (CC-5) and a lint
escape-hatch. No lib target, so integration tests are structurally awkward
(noted, not blamed).

## Findings

### 1. Runtime `sqlx::query(...)` + manual row extraction in auth  ·  #1  ·  med
- **Where:** `src/middleware/auth.rs:35` — `sqlx::query("SELECT
  discord_user_id FROM web_sessions WHERE token = $1 AND expires_at > now()")`
  then `r.get::<i64, _>("discord_user_id")`.
- **What / Why / Fix:** See [CC-5](_cross-cutting.md#cc-5). This is on the auth
  hot path — the strongest reason to move it to a checked `query_scalar!` (schema
  drift on `web_sessions` would then fail the build, not the request). Regenerate
  `.sqlx/` with `--all-features` afterward (per CLAUDE.md, so the `ssr`-gated
  server-fn queries are captured too).

### 2. `#[expect]` in login route  ·  #7  ·  low
- **Where:** `src/web/routes_login.rs:94`.
- **What / Why / Fix:** See [CC-3](_cross-cutting.md#cc-3).

### 3. Natural home for the bot's config/CRUD/display surface (CC-8 receiving end)  ·  #8  ·  med
- **Where:** `src/server/*` (mutation surface today: support/channels/roles/
  temp-voice/lfg settings, module toggles, tier), `src/ui/pages/*`.
- **What:** This crate is the destination for [CC-8](_cross-cutting.md#cc-8). The
  immediate, highest-value gap is the **duplicated** settings already written by
  both bot `setup` commands and existing server fns — the dashboard should become
  the single editor. Missing config pages: **music**, **ticket**,
  **suggestions**, **reaction-roles**. Missing read views: leaderboards
  (gambling/levels), destiny2 tier-list/loadout browse, palworld breed-plan.
- **Why it matters:** Convergence removes divergent write paths and moves
  data-dense views to a medium that fits them.
- **Suggested fix:** Prioritise (1) de-duplicating the settings the bot and web
  both write, (2) the four missing config pages against the existing
  `SettingsRegistry`, then (3) the read views. Each is its own scoped follow-up.

## Clean
- #1 Architecture: clean pages/components/server/web/middleware layering;
  rendering-mode code properly `#[cfg]`-gated (verified intentional per CLAUDE.md).
- #1 DB access: server-fn queries use compile-time macros (ssr-gated); only the
  auth middleware bypasses (finding #1).
- #2 Dead code: none found.
- #3 Async: no blocking I/O on request paths; no locks across `.await`.
- #6 Tests: no lib target → integration tests structurally awkward; acceptable,
  see [CC-6](_cross-cutting.md#cc-6).
