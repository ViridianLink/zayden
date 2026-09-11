# Zayden

Discord bot (`bot/`, `bot-modules/*`), Leptos dashboard (`dashboard/`), shared app layer (`zayden-app/`). Serenity, Postgres (`sqlx`), Rust 2024.

## Constraints

- **Toolchain**: Dev is nightly (`rust-toolchain.toml` pins it; bare `cargo` is nightly). CI/Docker use stable. Never pass `+nightly`.
- **Routing**: Manual Serenity routing and builders only. Never use `poise`.
- **Database**: `sqlx` compile-time macros only (`query!`, `query_as!`, `query_scalar!`). Never runtime `query()`.
- **Dependencies**: Workspace pins in root `Cargo.toml`. Crates set `dep.workspace = true` and may add features, never redefine versions. Do not downgrade. Run `cargo machete` when editing dependencies.
- **Lints**: Fix code instead of silencing lints. Use `#[expect(..., reason = "...")]` only when a code fix is demonstrably infeasible.

## Validation

**Do NOT run the validation suite on every task.** It is token and time-intensive. Run it only when explicitly requested by the user or at final milestone handoffs. For routine edits, rely on local background tools or narrow crate checks (`cargo check -p <crate>`).

When validation is requested:

- **Bacon** runs in the background. If `pgrep bacon` is empty, inform the user or fall back to raw commands. Touch a file after config/migration edits to prevent stale `.bacon-locations`.
- **Traps**:
    - Never use `--all-features` (`dashboard`'s `ssr` and `hydrate` conflict).
    - Clippy does not codegen or link; full build is required to verify compilation.
- **Full Validation Suite**:
    ```sh
    cargo fmt --check
    cargo clippy --workspace --exclude dashboard --all-targets -- -D warnings
    cargo clippy -p dashboard --features ssr -- -D warnings
    cargo clippy -p dashboard --target wasm32-unknown-unknown --features hydrate -- -D warnings
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
    cargo sqlx prepare --workspace -- --features ssr
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

- `design-docs/build-and-toolchain.md` — Toolchain, profile configurations, Cranelift notes.
- `.claude/skills/rust-skills/` — Rust guidelines.
- `.claude/agents/` — Perimeter subagents.
- `README.md` — Layout, build commands, Docker/CI setup.

## Multi-Agent Delegation Policy

Core (Opus): Architecture, algorithms, lifetimes, borrow-checking, state machines.  
Perimeter (Sonnet in `.claude/agents/`): High-volume, low-signal I/O (greps, test logs, diffs).

1. **`recon`**: Symbol searches, directory scans, call-graph tracing. Never dump large greps into core context.
2. **`planner`**: Scoping tasks and acceptance gates from `design-docs/` and manifests.
3. **`verifier`**: Test and validation execution. **Invoke ONLY when validation is explicitly requested or at final handoff**, not per-task. Ingest only `VERIFICATION_SUCCESS` or distilled failure blocks.
4. **`test-generator`**: Boilerplate test fixtures, tables, and mocks in `tests/` based on core-defined specifications.
5. **`git-hygiene`**: `cargo fmt`, diff auditing, explicit staging, Conventional Commit drafts. Never execute `git commit` or `git push` without explicit user confirmation.

**Escalation**: If a subagent fails twice on a task, execute the step directly in core.
