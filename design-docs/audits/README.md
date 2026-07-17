# Module-by-Module Audit Playbook

> A **repeatable procedure** for sweeping each crate in this workspace for
> quality and architectural issues. Written to be run **later, one module at a
> time** — this document is the method, not the audit itself.

## Purpose

The [implementation spec](../../docs/IMPLEMENTATION_PLAN.md) closed out the named
work items (destiny2 DB, config consolidation, enum migrations, temp-voice
panel). This playbook catches the *residual* issues those items didn't
target — dead code, stringly typing, async foot-guns, misplaced data, thin test
coverage — module by module, so the sweep is consistent and resumable.

## How to run it

- **One module per pass.** Pick a crate, answer the full checklist below, write
  the findings to `design-docs/audits/<module>.md`, stop. Do not batch multiple
  modules into one pass.
- **Audit, don't fix.** A pass *records* findings only. Fixes are separate,
  scoped follow-ups (each typically its own branch/PR), so the audit stays a
  fast read-only review and the fixes stay reviewable in isolation.
- **Explore-first.** Read the crate's `src/` tree and `tests/` before judging —
  layering and conventions only make sense in context.
- **Cite locations.** Every finding names `path:line` so the follow-up is
  actionable without re-deriving the context.

## Checklist (answer all 8 per module)

1. **Architecture & layering**
   - Are `commands` / `events` / `db` / `managers` separated into a clean module
     tree, or is logic tangled across files?
   - Does it follow the established conventions: `ModuleComponent` / `ModuleModal`
     traits for interaction routing, manager-trait structs for state, and (for
     settings) the `SettingsRow` / `SettingsStore` pattern?
   - Is DB access consistent — compile-time `query!` / `query_as!` macros
     throughout, one access pattern, no ad-hoc hand-written SQL bypassing the
     shared stores?

2. **Dead code & stubs**
   - `#[allow(dead_code)]` attributes, orphaned/unwired files, unused `pub` items.
   - Commented-out `// TODO` / `// FIXME` and stubbed match arms that silently do
     nothing. (Workspace lints deny `todo!()` / `unimplemented!()`, so those
     won't compile — look for the *soft* stubs instead.)

3. **Async correctness**
   - Blocking calls (`std::fs`, `std::thread::sleep`, blocking HTTP, CPU-bound
     loops) on async paths without `spawn_blocking`.
   - `unwrap()` / `expect()` on fallible I/O or network results.
   - Missing timeouts / cancellation on external calls.
   - Locks (`Mutex`/`RwLock` guards) held across an `.await`.

4. **Stringly typing & magic values**
   - String matching (`match s.as_str()`, `== "literal"`) that should be an enum
     with `FromStr` / `as_str` (and `sqlx::Type` when it's a DB column).
   - Hardcoded IDs, URLs, emoji, and other magic constants that belong in config
     or DB columns.

5. **Data placement**
   - `const` / static tables that should live in the DB or config — especially
     anything an admin might want to edit at runtime, or that other queries would
     benefit from filtering/joining. (Feeds future normalization work in the
     style of the destiny2 catalog migration.)

6. **Tests**
   - Present at all? Do they cover the module's real behavior, not just trivia?
   - Placed in `tests/` **integration files**, never inline `#[cfg(test)]` in
     `src/` (project convention).
   - For fixture-backed crates: are the fixtures current, and is regeneration
     documented?

7. **Lint hygiene**
   - Passes `cargo +nightly clippy --workspace --all-targets -- -D warnings`
     with **no** `#[allow]` / `#[expect]` escape hatches silencing warnings.
   - No unused dependencies (`cargo machete`).

8. **Dashboard suitability** _(the web dashboard is now live)_
   - Which commands/features would be **better served by the website** than by an
     in-bot slash command? The dashboard already owns guild settings (support,
     channels, roles, temp-voice, LFG), module enable/disable toggles, tiers, and
     — per the destiny2 decision — loadout CRUD. That precedent defines the
     heuristic.
   - **Move/mirror to the dashboard** (config, admin, CRUD, rich read-only views):
     one-shot `setup`/config commands that just write a `*_settings` row; admin
     CRUD of reference data (reaction-role maps, tags, catalog rows); and
     data-dense displays that a Discord embed renders poorly (leaderboards,
     profiles, tier lists, browsable catalogs). Flag anything **duplicating** a
     server function that already writes the same table.
   - **Keep in the bot** (anything requiring live Discord context): interactive
     gameplay, message/voice/reaction events, moderation actions, and per-message
     component flows (join/leave/kick/claim/playback) that only make sense against
     a live interaction, channel, or voice session.
   - Record candidates as: *feature → why the web fits better → what (if anything)
     stays in the bot*. This is a **direction** finding, not a defect — rank by
     duplication first, then user-experience gain.

## Output format (`design-docs/audits/<module>.md`)

```markdown
# Audit: <module>

_Audited: <date> · Commit: <short-sha>_

## Summary
<2–3 sentences: overall health, biggest concern.>

## Findings
### 1. <short title>  ·  <checklist #>  ·  <severity: high/med/low>
- **Where:** `path:line`
- **What:** <the issue>
- **Why it matters:** <impact>
- **Suggested fix:** <direction, not a full patch>

### 2. ...

## Clean
<Checklist items that passed, one line each — so re-audits can see what was
already verified.>
```

## Workspace coverage (all 20 crates)

The real workspace is `bot`, `zayden-app`, `dashboard`, and the 17
`bot-modules/*` crates. Two coverage corrections vs. the original draft of this
list:

- **Added** four crates the first draft omitted: `ai`, `gold-star`, `llamad2`,
  and the top-level **`bot`** binary itself (where every `ModuleComponent` /
  `ModuleModal` binding and all the `moderation` code actually lives).
- **`moderation` is not a crate.** It is a set of bindings under
  `bot/src/bindings/moderation/` — audited as part of **`bot`**, not as a
  standalone module.

Read [`_cross-cutting.md`](_cross-cutting.md) first: it records the
workspace-wide themes (the DB-generic `async_trait` manager pattern, inline
`#[cfg(test)]`, `#[expect]` inventory, runtime-SQL bypasses) once, so the
per-module files can reference them instead of repeating them.

## Suggested order

Start with the crates the implementation spec touched (regression-adjacent,
freshest in context):

1. `destiny2`
2. `temp-voice`
3. `music`
4. `ticket`
5. `config` (zayden-app)

Then breadth-first across the rest:

`marathon` · `gambling` · `lfg` · `levels` · `palworld` · `reaction-roles` ·
`suggestions` · `verify` · `family` · `zayden-core` · `ai` · `gold-star` ·
`llamad2` · `dashboard` · `bot`

## Progress

| Module | Audited | File |
|--------|---------|------|
| _cross-cutting_ | ☑ | [_cross-cutting.md](_cross-cutting.md) |
| destiny2 | ☑ | [destiny2.md](destiny2.md) |
| temp-voice | ☑ | [temp-voice.md](temp-voice.md) |
| music | ☑ | [music.md](music.md) |
| ticket | ☑ | [ticket.md](ticket.md) |
| config (zayden-app) | ☑ | [config.md](config.md) |
| marathon | ☑ | [marathon.md](marathon.md) |
| gambling | ☑ | [gambling.md](gambling.md) |
| lfg | ☑ | [lfg.md](lfg.md) |
| levels | ☑ | [levels.md](levels.md) |
| palworld | ☑ | [palworld.md](palworld.md) |
| reaction-roles | ☑ | [reaction-roles.md](reaction-roles.md) |
| suggestions | ☑ | [suggestions.md](suggestions.md) |
| verify | ☑ | [verify.md](verify.md) |
| family | ☑ | [family.md](family.md) |
| zayden-core | ☑ | [zayden-core.md](zayden-core.md) |
| ai | ☑ | [ai.md](ai.md) |
| gold-star | ☑ | [gold-star.md](gold-star.md) |
| llamad2 | ☑ | [llamad2.md](llamad2.md) |
| dashboard | ☑ | [dashboard.md](dashboard.md) |
| bot | ☑ | [bot.md](bot.md) |
