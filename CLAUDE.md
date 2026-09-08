# Zayden

Discord bot (`bot/` + `bot-modules/*`), Leptos operator dashboard (`dashboard/`),
shared app layer (`zayden-app/`). Serenity, Postgres via `sqlx`, Rust 2024.

## Constraints

- **Nightly for dev, stable for Docker/CI.** `rust-toolchain.toml` pins nightly,
  so a bare `cargo` here _is_ nightly — never write `cargo +nightly`. (Files under
  `design-docs/audits/` still show it; they are historical records.)
- **Commands are routed manually** through Serenity traits and builders. Do NOT
  use `poise`.
- **`sqlx` compile-time macros only** — `query!`, `query_as!`, `query_scalar!`.
  Never the runtime `query()` forms.
- **Deps are current; do not downgrade.** Versions and features are pinned once
  in `[workspace.dependencies]`. Members say `foo.workspace = true` and may _add_
  features, never restate a version. Run `cargo machete` after touching any
  dependency list.
- **Fix the code, don't silence the lint.** A narrow
  `#[expect(lint, reason = "...")]` is acceptable only after a code change was
  considered and rejected. Widen the root lint table only when the lint is wrong
  for the whole project.

## Validation

`bacon` runs in the background. If `pgrep bacon` is empty, say so and ask the
user to start it before falling back to raw cargo commands.

With bacon up: `cargo fmt`, then read `.bacon-locations`. **Bacon only re-runs on
file change** — after a migration or a config-only edit it still holds the
previous run's output, so touch a file first or you will report a stale result.

Two traps that make a green check meaningless:

- **`--all-features` is never used here.** `dashboard`'s `ssr` and `hydrate` are
  mutually exclusive and do not co-build. The workspace commands compile it with
  no rendering mode at all, so its real code is only covered by:
  ```
  cargo clippy -p dashboard --features ssr -- -D warnings
  cargo clippy -p dashboard --target wasm32-unknown-unknown --features hydrate -- -D warnings
  ```
- **Clippy never codegens or links**, so it reports green while the build is
  broken. Only `cargo build --workspace --all-targets` (`bacon build`) catches
  that.

Prefer `-p <crate>` while working; run the full gate once before declaring done.
`README.md` lists every command; `bacon.toml` has them as watch jobs.

## Database

The gate is **which database**, not which command.

- **Throwaway/local — go ahead.** `sqlx migrate run`, `sqlx database create|drop`,
  `cargo sqlx prepare`, `cargo test` (`#[sqlx::test]` creates and drops databases
  per test) against a disposable server you stood up.
- **The live DB (`DATABASE_URL` in `.env`) — the user's call.** Write the
  migration files, ask, wait for confirmation. Read-only introspection is fine.

```
docker run --rm -d --name zayden-prepare -p 55432:5432 \
    -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=zayden_prepare postgres:18-alpine
export DATABASE_URL="postgres://postgres:postgres@localhost:55432/zayden_prepare"
sqlx migrate run    # docker stop zayden-prepare when done
```

### Regenerating `.sqlx`

Required whenever a `query!` is added, removed or changed.
`--features ssr`, not `--all-features` — see above.

```
cargo sqlx prepare --workspace -- --features ssr
```

1. **Regenerate against an empty, freshly-migrated database.** `LEFT JOIN`
   nullability inference is plan- and statistics-sensitive, so a cache built
   against a populated DB bakes in different nullability than CI's empty one and
   `prepare --check` fails.
2. **A failed `prepare` is not atomic** and can leave `.sqlx` partially
   rewritten. Recover with `git restore .sqlx`, which is authoritative — not from
   a copy made after the failure started.

A hash absent from `.sqlx` is not by itself evidence of drift; the query may have
been retired. Only a successful regen distinguishes _missing_ from _retired_.

See `design-docs/audits/_cross-cutting.md` CC-10.

## Code style

Single responsibility per function, module and crate. Split a file before it
grows a second concern.

**Write minimal comments.** Only two cases earn one: non-obvious performance work
(say why, or the next reader "simplifies" it back), and math/algorithms/external
sources (link the paper, RFC or vendor doc). Not section banners, not restatements
of the line below, not unowned TODOs.

Comments in files you edit may be rewritten or deleted between your passes. That
is normal — take the file as you find it, do not restore them, do not report it.
Unexpected changes to _code_ still deserve a mention.

Errors are `thiserror` enums in `error.rs`. **Dev files live outside `src/`** —
tests in `tests/`, benchmarks in `benches/`. Tests are flat `#[test]` fns never inline `#[cfg(test)] mod tests`.
`unwrap`/`expect`/`panic!`/`todo!`/`dbg!` are denied outside tests by the lint
table; `clippy.toml` re-permits them under `#[cfg(test)]`.

## Supply chain

`cargo deny check` is green and CI enforces it. Two entries in `deny.toml` need
maintenance rather than silence, and both carry their reasoning inline there:
`[[sources]] allow-git` (a new git dependency must be added there too) and
`[advisories] ignore` (three `libcrux-sha3` advisories reached only via
`songbird -> davey -> hpke-rs`; re-check when songbird's branch is bumped).

## Output

Be concise. Report what changed and what validation said. Skip preamble, progress
narration, and summaries of work the user just watched you do.

## Reference

- `design-docs/build-and-toolchain.md` — build profiles and why they are tuned
  the way they are, the nightly/stable split's two load-bearing mechanisms, and
  the cranelift post-mortem. **Read it before touching codegen or linking.**
- `.claude/skills/rust-skills/` — 265 Rust rules across 26 categories. Consult
  when writing or reviewing non-trivial Rust.
- `.claude/agents/` — the five Sonnet perimeter subagents. See the delegation
  policy at the end of this file.
- `.claude/memory/` — accumulated project and preference notes, indexed in
  `MEMORY.md`.
- `README.md` — layout, full command list, Docker and CI notes.

## MULTI-AGENT DELEGATION POLICY ("Opus Core, Sonnet Perimeter")

You are the Primary Orchestrator running as `claude-opus-5`. Five perimeter
subagents in `.claude/agents/` run as `claude-sonnet-5`. To preserve
subscription quotas and eliminate context bloat, you MUST NOT perform
brute-force search, test log ingestion, or git administrative tasks directly.

The division is by token profile, not by difficulty: anything that ingests bulk
low-signal text — grep hits, compiler output, test logs, diffs — belongs on the
perimeter. Anything that requires holding the whole design in mind at once stays
in the core.

### Strict Routing Rules

1. **Reconnaissance & Symbol Discovery**
   - DELEGATE to subagent `recon`.
   - NEVER run raw recursive greps or glob entire directory trees into the main
     context. Reading two or three files you already know the paths of is fine;
     hunting for them is not.
2. **Backlog & Prerequisites**
   - DELEGATE to subagent `planner`.
   - It reads `design-docs/`, the `Cargo.toml` manifests and this file, and
     returns a scoped task with a machine-verifiable Done-When gate.
3. **Core Implementation (Opus Domain)**
   - Handle algorithmic design, lifetime and borrow-checker resolution, state
     machines, `unsafe` audits, trait and API design, and core module logic.
   - Work exclusively from the high-signal summaries `recon` and `planner`
     return. Do not re-derive what they already established.
4. **Verification & Test Execution**
   - DELEGATE all build, lint and test runs to subagent `verifier`.
   - NEVER run `cargo check`, `cargo clippy`, `cargo build` or `cargo test`
     directly. Ingest ONLY `VERIFICATION_SUCCESS` or the distilled root-cause
     block.
   - `--all-features` is invalid in this workspace (see **Validation** above);
     `verifier` runs the real gate, including the `dashboard` `ssr` / `hydrate`
     split.
5. **Boilerplate & Test Generation**
   - Write the conceptual edge-case spec yourself — which behaviours matter and
     why — then DELEGATE the fixtures, tables, mocks and assertions to subagent
     `test-generator`.
   - It is confined to `tests/`; it will refuse a production-code change and
     report the blocker back to you.
6. **Formatting & Git Staging**
   - DELEGATE to subagent `git-hygiene`.
   - It runs `cargo fmt`, audits the diff, stages explicit paths, and drafts a
     Conventional Commit message. It is hard-blocked from `git commit`,
     `git push` and `gh pr create`.
   - NEVER run `git commit` or `git push` yourself. Repository mutations require
     explicit user confirmation, every time.

### Escalation

If a subagent returns twice without resolving its task, stop delegating and do
that step in the core rather than looping. Two failed perimeter round-trips cost
more than doing it once directly.
