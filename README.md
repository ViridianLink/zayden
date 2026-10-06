# Zayden

A Discord bot, an operator dashboard, and the shared application layer between
them. Serenity for Discord, Leptos for the dashboard, Postgres via `sqlx`.

## Layout

| Path                      | What it is                                                      |
| ------------------------- | --------------------------------------------------------------- |
| `bot/`                    | the bot binary — wires modules into a Serenity client           |
| `bot-modules/*`           | one crate per feature (gambling, levels, music, palworld, …)    |
| `bot-modules/zayden-core` | command/event routing traits shared by every module             |
| `zayden-app`              | application layer: config, entitlements, shared state           |
| `dashboard/`              | Leptos SSR + WASM hydration operator dashboard                  |
| `web/`                    | Topcoat server-rendered operator dashboard                      |
| `migrations/`             | sqlx migrations, immutable once written                         |
| `.sqlx/`                  | offline query cache; committed, regenerated on any query change |
| `design-docs/`            | specs, plans, and per-module audits                             |

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
bacon                    # clippy, the default job
bacon test               # cargo test
bacon fmt                # cargo fmt --all --check
bacon build              # link check
bacon check-ssr          # dashboard, server side
bacon check-hydrate      # dashboard, wasm32 side
bacon check-web          # web, check
bacon check-offline-web  # web, check with SQLX_OFFLINE=true
bacon clippy-web         # web, clippy
bacon test-web           # web, tests
bacon bundle-web         # web, topcoat asset bundle
bacon nextest             # needs `cargo install cargo-nextest`
```

Keep the flags identical across this README, `bacon.toml` and CI. A local
command weaker than CI is how drift starts.

## Web dashboard (`web/`)

`web` is a [Topcoat](https://github.com/tokio-rs/topcoat) 0.10 app: every page
renders on the server, and `web/src/router.rs` registers every layout, page and
route in one `router()` function that the binary and the `tests/` harness share.
It reads the same `bot/config.toml` `[dashboard]` keys and environment as
`dashboard`.

It needs the Topcoat CLI, pinned to the same version as the crate:

```bash
cargo install topcoat-cli@0.10.0 --locked
```

Run it from the workspace root against a throwaway database:

```bash
topcoat asset bundle -p web
DATABASE_URL=postgres://postgres:postgres@localhost:55432/zayden_prepare \
DASHBOARD_BIND_ADDR=127.0.0.1:3100 \
cargo run -p web
```

- **`topcoat asset bundle -p web`** builds the binary and writes its asset
  bundle (the compiled stylesheet and every other `asset!`) to `assets/` beside
  it, e.g. `target/debug/assets/`. The server refuses to start without a bundle,
  and a bundle only matches the build it came from: re-run the command after any
  change, before `cargo run`. `--release` bundles the release build. The CLI
  clears `RUSTUP_TOOLCHAIN` and `RUSTFLAGS` for its inner build, so it always
  builds with the toolchain `rust-toolchain.toml` selects.
- **`DASHBOARD_BIND_ADDR`** overrides `[dashboard].bind_addr`, so `web` can run
  next to `dashboard` on `:3000`. Discord OAuth redirects are registered for
  `:3000`, so log-in flows only complete when `web` runs there.
- **`TAILWIND_CLI`** points `web/build.rs` at a Tailwind CLI binary (v4.3.2).
  Unset, the build downloads that release itself (about 112 MB) and, on
  x86-64 Linux, checks it against a pinned SHA-256; on nightly the download
  repeats whenever the build script's output directory changes. Point it at a
  local copy to build offline; CI and `docker/Dockerfile.web` always set it to a
  checksummed download.
- Styles live in `web/style/` (`input.css` plus partials). Tailwind only sees
  class names written literally in `web/src/**/*.rs`.
- `view!` bodies are formatted by `topcoat fmt`, which has no `--check` mode;
  pass explicit paths (`topcoat fmt web/src web/tests`) so it skips `target/`,
  then run `cargo fmt`.

### Dev loop

```bash
export TAILWIND_CLI=/path/to/tailwindcss-4.3.2     # optional, see above
topcoat asset bundle -p web                        # after every change
cargo run -p web
topcoat fmt web/src web/tests && cargo fmt         # format view! bodies, then the rest
```

`bacon bundle-web` runs the bundle step, `bacon check-web` and `bacon clippy-web`
watch the crate. `cargo run` alone serves the bundle from the last
`topcoat asset bundle`, so a stale bundle shows stale CSS or a missing-asset
error.

### Tests

```bash
cargo test -p web
```

Most files in `web/tests/` render pages through the real router. The ones that
touch Postgres use `#[sqlx::test]`, which needs `DATABASE_URL` pointing at a
**throwaway** server (see Database below); `cargo test` creates and drops one
database per test. Nothing in `web` is feature-gated, so the plain workspace
commands run every test. `tests/legacy_*.rs` hold the source and stylesheet
scans carried over from the `dashboard` crate.

### Docker

`docker/Dockerfile.web` builds the release image on stable: a separate build target
downloads the Tailwind CLI and checks its SHA-256, the builder installs
`topcoat-cli`, cooks the dependencies with cargo-chef, removes
`rust-toolchain.toml` and runs `topcoat asset bundle --release -p web`, and the
runtime image copies `/app/web` and `/app/assets` side by side (the binary
looks for `assets/` next to itself). It runs as the `zayden` user, listens on
`0.0.0.0:3000` (`DASHBOARD_BIND_ADDR`) and reads `/app/config.toml` like the
dashboard image.

```bash
docker build -f docker/Dockerfile.web -t zayden-web .
```

The image is not in the publish matrix and has no `docker-compose.yml`
service yet; CI only builds it.

## Conventions

- Lints are declared once in the root `Cargo.toml` under `[workspace.lints]`;
  every crate opts in with `[lints] workspace = true`.
- Dependency versions are pinned once in `[workspace.dependencies]`; member
  crates say `foo.workspace = true` and may _add_ features on top, but never
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

`.github/workflows/ci.yml` runs these jobs in parallel: `rustfmt`, workspace
`clippy`, the dashboard's two extra clippy targets (`ssr` and wasm32 `hydrate`),
workspace tests against a Postgres service, `cargo-deny`, and
`sqlx prepare --check`. `web` is covered by the workspace `clippy` and `test`
jobs; `web-bundle` runs `topcoat asset bundle -p web` and `web-image` builds
`docker/Dockerfile.web` without pushing. Jobs that compile `web` set
`TAILWIND_CLI` to a checksummed Tailwind download. Everything builds on **stable**
(`RUSTUP_TOOLCHAIN=stable` outranks `rust-toolchain.toml`, and `RUSTFLAGS=""`
clears the nightly-only flags in `.cargo/config.toml`), so the CI format gate is
weaker than `cargo fmt` locally — stable `rustfmt` ignores the unstable options
rather than erroring.

The `images` job in the same workflow builds the `bot` and `dashboard` release
images once `rustfmt`, both dashboard clippy jobs and `cargo-deny` pass, and
pushes them to GHCR for every event except pull requests.

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

`docker/Dockerfile.bot`, `docker/Dockerfile.dashboard` and `docker/Dockerfile.web` are cargo-chef builds:
dependencies are cooked into a cached layer, then the binary is compiled and
copied into a `debian:trixie-slim` runtime that runs as a non-root `zayden`
user. Both build on **stable** — `.dockerignore` keeps `rust-toolchain.toml` and
`.cargo/` out of the context, so anything in the manifests must parse on stable
cargo.

`.sqlx/`, `migrations/`, `config.toml` and `radio.toml` are deliberately in the
build context; almost everything else, including `.claude/`, is not.
