# Audit: zayden-core

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

The shared foundation crate: `Ctx`, `Module`/`ModuleComponent`/`ModuleModal`
traits, cron scaffolding, cache, scope/snowflake/format helpers, templates. It is
where the generic `<Db: Database>` trait bounds that propagate into the manager
crates originate (`cron.rs`, `events.rs`, `module.rs`), so it is both a CC-1
*source* and the place a de-generalisation would start. One inline test module.

## Findings

### 1. Generic `<Db: Database>` trait bounds in core traits  ·  #1  ·  med
- **Status:** `in-review`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-24):** Final CC-1 piece — de-generalised the `zayden-core` core
  traits now that every module manager is concrete (`gambling` was the last,
  `83930148`). `cron.rs`: dropped `<Db: Database>` from `CronJob`, `CronJobData`
  and `ActionFn`, pinning them to `PgPool` (the only DB). `events.rs`: **deleted**
  — the seven `run<Db: Database>(pool: &Pool<Db>)` event traits (`GuildCreate`,
  `MessageCreate`, `ReactionAdd`, `ReactionRemove`, `Ready`, `VoiceStateUpdate`,
  `ThreadDelete`) were dead code (grep-verified: no `impl`, no `use`, only
  `pub mod events;` declared them — event routing is done directly in
  `bot/src/handler/*`), so removing the file drops both their Db-generic and their
  `#[async_trait]` by deletion; also removed `pub mod events;` from `lib.rs`.
  Collapsed every downstream `CronJob<Postgres>` / `CronJobData<Postgres>` /
  `ActionFn<Postgres>` / `CronJob::<Postgres>::new` / `<Data: CronJobData<Postgres>>`
  to the non-generic form across `bot` (`cron.rs`, `state.rs`), `destiny2`,
  `gambling` (`stamina`/`higherlower`/`lotto`), `palworld`, `marathon`, and `lfg`
  (`events.rs`, `cron/reminders.rs`, `modals/{create,edit}.rs`) and trimmed the
  now-unused `Postgres` imports. **No behaviour change** (pure type-parameter
  removal, single-DB); no `.sqlx`/`Cargo.toml` delta (`async_trait` is still used
  by `module.rs`, so the dep stays). Verified: `cargo check` + `cargo +nightly
  clippy --workspace --all-targets -D warnings` clean, `cargo test` green, no new
  `#[allow]`/`#[expect]`. **Residual:** `module.rs`'s `#[async_trait]` on
  `ModuleCommand`/`Component`/`Modal`/`Autocomplete` is left as-is — those traits
  carry **no** Db generic (already ctx-based) and are `dyn`-dispatched in the
  registry, so native async-fn-in-traits (not `dyn`-compatible) can't replace
  `async_trait` there without a boxed-future shim; that is a separate concern from
  this Db-generic finding. The `zayden-core` `CronJob<Db>` generic is now gone, so
  the only remaining CC-1 note workspace-wide is this `module.rs` `async_trait`
  residual. No regression test: this is a compile-time type change with no runtime
  behaviour or pure-logic surface — the workspace build + existing suite are the
  net.
- **Where:** `src/cron.rs`, ~~`src/events.rs`~~ (deleted), `src/module.rs`.
- **What:** The core `Module`/cron/event traits are generic over the sqlx
  `Database`, which is what forces every downstream manager to be generic too
  (the root of CC-1).
- **Why it matters:** As long as core stays generic over `Db`, the manager
  crates can't cleanly go concrete. This is the *keystone* of the CC-1 migration
  — de-generalising here (to `Postgres`) unblocks all the module-level cleanups.
- **Suggested fix:** Plan the CC-1 migration top-down: pin the core traits to
  `Postgres` first, then convert managers crate-by-crate. See
  [CC-1](_cross-cutting.md#cc-1).

### 2. Inline `#[cfg(test)]` module  ·  #6  ·  low
- **Where:** `src/snowflake.rs:13`.
- **What / Why / Fix:** See [CC-2](_cross-cutting.md#cc-2). Move to `tests/`.

## Clean
- #2 Dead code: none found.
- #3 Async: cache/cron helpers non-blocking; no locks across `.await` observed.
- #4 Stringly typing: `format_num`/`scope`/`templates` are typed helpers.
- #7 Lint: no `#[expect]`/`#[allow]`.
