# Audit: dashboard

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Leptos full-stack crate (SSR + WASM hydrate) with a clean `ui/pages` +
`ui/components` + `server/` + `web/` + `middleware/` split. Rendering-mode code is
correctly `#[cfg(feature = "...")]`-gated so the default clippy pass stays clean.
One issue: a lint escape-hatch. No lib target, so integration tests are
structurally awkward (noted, not blamed).

## Findings

### 2. `#[expect]` in login route  ·  #7  ·  low
- **Status:** `unclear`
- **Where:** `src/web/routes_login.rs:94`.
- **What / Why / Fix:** See [CC-3](_cross-cutting.md#cc-3).

### 3. Natural home for the bot's config/CRUD/display surface (CC-8 receiving end)  ·  #8  ·  med
- **Status:** `unclear`
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
- #1 DB access: server-fn queries use compile-time macros (ssr-gated), as does
  the auth middleware.
- #2 Dead code: none found.
- #3 Async: no blocking I/O on request paths; no locks across `.await`.
- #6 Tests: no lib target → integration tests structurally awkward; acceptable,
  see [CC-6](_cross-cutting.md#cc-6).

## Addendum — website roles and the Palworld save editor (2026-07-28)

**`web_user_roles`** (migration `0021`) is the dashboard's first website-level
role table: `(discord_user_id, role)`, seeded with the bot owner as `admin`
because there is otherwise no way to grant the first role. It is deliberately
distinct from `guild_admin_context`, which checks a Discord guild permission
bitfield over the API — this checks a row we own. `require_role(WebRole::Admin)`
in `server/auth.rs` is the server-fn gate; it returns `"unauthenticated"` with
no valid session and `"forbidden"` when the session is valid but the role is
absent.

**`/admin/palworld/save`** is an unlinked admin page. It appears in no
navigation, and *any* error from `get_save_roster` — no session, no role —
renders the `NotFound` view rather than an error or a 403, so the page does not
disclose its own existence. The export route (`POST
/admin/palworld/save/export`) re-checks `web_user_roles` itself, because the
`require_auth` middleware proves a session but not a role, and returns `404`
rather than `403` for the same reason. A genuine configuration error ("no world
save is configured") is shown as itself, since reaching it already proves the
caller is an admin.

Two structural notes:

- The save DTOs (`SaveRoster`, `SavePal`, `SaveEdits`, …) are **mirrored** in
  `dashboard::dto`, not re-exported from the `palworld` crate. `palworld` is an
  `ssr`-only dependency, but these types appear in a `#[server]` signature and
  are constructed client-side by the editor page, so the WASM build needs them.
  Conversion to and from `palworld::save::edit` happens at the server boundary
  behind `#[cfg(feature = "ssr")]`.
- The editor is stateless between load and export. A parsed world is hundreds of
  megabytes, so nothing is cached; the export re-reads the mirror. The mirror is
  opened read-only on every path and nothing is written back to it or to the
  game server — the only output is an HTTP download.
