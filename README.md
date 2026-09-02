# Zayden

A Discord bot, an operator dashboard, and the shared application layer between
them. Serenity for Discord, Leptos for the dashboard, Postgres via `sqlx`.

## Layout

| Path | What it is |
| --- | --- |
| `bot/` | the bot binary — wires modules into a Serenity client |
| `bot-modules/*` | one crate per feature (gambling, levels, music, palworld, …) |
| `bot-modules/zayden-core` | command/event routing traits shared by every module |
| `zayden-app` | application layer: config, entitlements, shared state |
| `dashboard/` | Leptos SSR + WASM hydration operator dashboard |
| `migrations/` | sqlx migrations, immutable once written |
| `.sqlx/` | offline query cache; committed, regenerated on any query change |
| `design-docs/` | specs, plans, and per-module audits |

## Commands

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo fmt --all
```

`--all-targets` defaults to _off_ and silently skips tests, benches and
examples. Always pass it.

`--all-features` is deliberately **not** used here, unlike the template this
project tracks: `dashboard` exposes mutually exclusive `ssr` and `hydrate`
features and does not build with both on. Cover it with its own commands:

```bash
cargo clippy -p dashboard --features ssr -- -D warnings
cargo clippy -p dashboard --target wasm32-unknown-unknown --features hydrate -- -D warnings
```

Clippy never codegens or links, so it reports green while the build is broken.
Only a real build exercises linking:

```bash
cargo build --workspace --all-targets
```

`bacon.toml` defines all of the above as watch jobs:

```bash
bacon                  # clippy, the default job
bacon test             # cargo test
bacon fmt              # cargo fmt --all --check
bacon build            # link check
bacon check-ssr        # dashboard, server side
bacon check-hydrate    # dashboard, wasm32 side
bacon nextest          # needs `cargo install cargo-nextest`
```

Keep the flags identical across this README, `bacon.toml` and CI. A local
command weaker than CI is how drift starts.

## Conventions

- Lints are declared once in the root `Cargo.toml` under `[workspace.lints]`;
  every crate opts in with `[lints] workspace = true`.
- Dependency versions are pinned once in `[workspace.dependencies]`; member
  crates say `foo.workspace = true` and may *add* features on top, but never
  restate a version. Unused workspace entries cost nothing — cargo only
  resolves a dependency once a member opts in.
- `unwrap`/`expect`/`panic`/`todo`/`dbg!` are **denied** outside tests;
  `clippy.toml` re-permits them under `#[cfg(test)]`.
- Tests live in `tests/`, not in inline `#[cfg(test)] mod tests` blocks.
- Commands are registered and routed manually through Serenity traits. `poise`
  is not used.
- `rustfmt.toml` uses unstable options, so `rust-toolchain.toml` pins nightly
  for local work. Never write `cargo +nightly` — the prefix is redundant.

## Toolchain

Nightly for local development, stable for release images. See
`design-docs/build-and-toolchain.md` for the two mechanisms that keep the split
working (`Cargo.toml` must stay stable-parseable; `.dockerignore` must keep
excluding `.cargo/`), how the build profiles are tuned, and why the cranelift
codegen backend is not used.

## CI

`.github/workflows/ci.yml` runs `rustfmt`, `clippy`, the dashboard's two extra
targets, tests, `cargo-deny` and an `sqlx prepare --check` in parallel, then a
link-checking `build` gated on the first three. It builds on **stable**
(`RUSTUP_TOOLCHAIN=stable` outranks `rust-toolchain.toml`, and `RUSTFLAGS=""`
clears the nightly-only flags in `.cargo/config.toml`), so the CI format gate is
weaker than `cargo fmt` locally — stable `rustfmt` ignores the unstable options
rather than erroring.

`.github/workflows/docker-publish.yml` builds and pushes the two release images
to GHCR on pushes to `main` and on tags.

## Database

```bash
cp .env.example .env
sqlx database create
sqlx migrate run
cargo sqlx prepare --workspace -- --features ssr
```

**Only ever run mutating `sqlx` commands against a throwaway database.** The
`DATABASE_URL` in `.env` points at the live one; migrations there are the user's
call. Stand up a disposable server first:

```bash
docker run --rm -d --name zayden-prepare -p 55432:5432 -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=zayden_prepare postgres:18-alpine
```

`.sqlx/` must be committed: CI sets `SQLX_OFFLINE=true` and never connects to a
database, so that cache is the only thing the query macros have to check
against. Regenerate it against an **empty, freshly-migrated** database — `LEFT
JOIN` nullability inference is plan-sensitive, and a cache built against a
populated DB will fail CI's `prepare --check`.

## Docker

```bash
docker compose up --build
```

`docker/Dockerfile.bot` and `docker/Dockerfile.dashboard` are cargo-chef builds:
dependencies are cooked into a cached layer, then the binary is compiled and
copied into a `debian:trixie-slim` runtime that runs as a non-root `zayden`
user. Both build on **stable** — `.dockerignore` keeps `rust-toolchain.toml` and
`.cargo/` out of the context, so anything in the manifests must parse on stable
cargo.

`.sqlx/`, `migrations/`, `config.toml` and `radio.toml` are deliberately in the
build context; almost everything else, including `.claude/`, is not.
