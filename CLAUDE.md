# Zayden

Discord bot (`bot/`, `bot-modules/*`), Topcoat dashboard (`web/`), shared app layer (`zayden-app/`). Serenity, Postgres (`sqlx`), Rust 2024.

## Constraints

- **Toolchain**: Dev is nightly (`rust-toolchain.toml` pins it; bare `cargo` is nightly). CI/Docker use stable. Never pass `+nightly`.
- **Routing**: Manual Serenity routing and builders only. Never use `poise`.
- **Database**: `sqlx` compile-time macros only (`query!`, `query_as!`, `query_scalar!`). Never runtime `query()`.
- **Dependencies**: Workspace pins in root `Cargo.toml`. Crates set `dep.workspace = true` and may add features, never redefine versions. Do not downgrade. Run `cargo machete` when editing dependencies.
- **Topcoat**: Pre-1.0 and volatile. Write Topcoat code only from `docs/topcoat/` (version-pinned, verified entries) or the pinned crate source, never from memory. Keep that reference in step when bumping Topcoat: its `Verified against` line must match `Cargo.lock`.
- **Lints**: Fix code instead of silencing lints. Use `#[expect(..., reason = "...")]` only when a code fix is demonstrably infeasible.

## Validation

**Do NOT run the full validation suite on every task.** It is token and time-intensive. Run it only when explicitly requested by the user or at final milestone handoffs. For routine edits, rely on local background tools or narrow checks (`cargo check -p <crate>`, `cargo test -p <crate> <filter>` for the tests you touched).

When validation is requested:

- **Bacon** runs in the background. If `pgrep bacon` is empty, inform the user or fall back to raw commands. Touch a file after config/migration edits to prevent stale `.bacon-locations`.
- **Traps**:
  - Clippy does not codegen or link; full build is required to verify compilation.
- **Full Validation Suite**:
  ```sh
  cargo fmt --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo build --workspace --all-targets
  cargo test --workspace
  ```

## Database

- **Local/Throwaway**: Free to run migrations, create/drop DBs, and run `cargo test` / `cargo sqlx prepare`.
  ```sh
  docker run --rm -d --name zayden-prepare -p 55432:5432 -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=zayden_prepare postgres:18-alpine
  export DATABASE_URL="postgres://postgres:postgres@localhost:55432/zayden_prepare"
  sqlx migrate run
  ```
- **Live DB (`.env`)**: Never mutate without explicit user confirmation. Read-only queries are fine.
- **Regenerating `.sqlx`**: Required when `query!` changes. Run against an empty, freshly-migrated DB (plan-sensitive nullability inference):
  ```sh
  cargo sqlx prepare --workspace -- --all-targets
  ```
  If prepare fails, revert via `git restore .sqlx`.

## Code Style

- Single responsibility per function, module, and crate.
- **Minimal comments**: Only explain non-obvious performance optimizations or cite algorithms/specs. No banners, paraphrasing, or unassigned TODOs.
- **Errors**: `thiserror` enums placed in `error.rs`.
- **Tests & Dev**: Dev files live outside `src/` (tests in `tests/`, benchmarks in `benches/`). Flat `#[test]` functions; never inline `#[cfg(test)] mod tests`. `unwrap`/`expect`/`panic!` forbidden outside tests.

## Supply Chain & Output

- `cargo deny check` must pass. New git deps require an entry in `deny.toml` `[[sources]] allow-git`.
- **Output**: Concise. Report what changed and verification output. No conversational preamble or summaries of obvious steps.

## References

- `docs/build-and-toolchain.md` — Toolchain, profile configurations, Cranelift notes.
- `docs/topcoat/` — gitignored symlink to the shared `~/Documents/Reference/topcoat/`, the Topcoat reference for the pinned version: core APIs, UI components, tooling, release changes.
- `.claude/skills/rust-skills/` — Rust guidelines.
- `README.md` — Layout, build commands, Docker/CI setup.

## Agent notes

The global agent workflow applies. Project specifics:

- **`code-builder`**: any TDS touching `web/` must cite the `docs/topcoat/` sections (or pinned crate source paths) for every Topcoat API it uses; for bot code, specify the Serenity builders explicitly (no `poise`). A TDS that changes a `query!` must say so; regenerate `.sqlx/` yourself afterwards.
- **`test-runner`**: give it narrow `-p <crate>` commands while iterating; the full suite only when requested or at milestone handoff.
- **`repo-scout`**: use for `docs/topcoat/` lookups and cross-module questions across `bot-modules/*`.
