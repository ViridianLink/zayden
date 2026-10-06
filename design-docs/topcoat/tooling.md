Verified against: topcoat 0.10.0 (Cargo.lock), 2026-10-06

# Tooling: the `topcoat` CLI, Tailwind, fonts, icons and the `web` build

Everything here was read from `topcoat-cli-0.10.0`, `topcoat-ui-0.10.0`, `topcoat-tailwind-0.10.0`, `topcoat-font-0.10.0`, `topcoat-icon-0.10.0`, `topcoat-asset-0.10.0` and `topcoat-core-0.10.0`, run through the installed `topcoat 0.10.0` binary, or compiled in the probes under `scratch/topcoat-kb/ui/`. Components, theme and icon recipes are in [ui.md](ui.md).

## Contents

- [The CLI](#the-cli)
- [Environment variables](#environment-variables)
- [Output and cache directories](#output-and-cache-directories)
- [Tailwind build integration](#tailwind-build-integration)
- [Font and icon modules](#font-and-icon-modules)
- [How this repository builds `web`](#how-this-repository-builds-web)
- [What enabling fonts and icons changes](#what-enabling-fonts-and-icons-changes)
- [Corrections to older notes](#corrections-to-older-notes)

## The CLI

Install and version pin: `cargo install topcoat-cli@0.10.0 --locked` installs two binaries, `topcoat` and `cargo-topcoat` (`cargo topcoat <command>` is equivalent; the cargo shim drops the leading `topcoat` argument). `topcoat --version` and `-V` exist since 0.10. Before every command the CLI reads the nearest `Cargo.lock` at or above the working directory and prints a warning (never an error) when the locked `topcoat` version is not in the CLI's semver-compatibility group (for 0.10.x: `0.10`). `TOPCOAT_NO_VERSION_CHECK` set to any value silences it.

Status: VERIFIED (`topcoat <command> --help` and the source).

| Command | Purpose |
|---|---|
| `topcoat dev` | Build, bundle assets and run the app; rebuild on source changes. |
| `topcoat fmt [FILES]...` | Format the bodies of Topcoat macros (and optionally the Rust around them). |
| `topcoat asset list` | Print the asset sources embedded in the built binary. |
| `topcoat asset bundle` | Build the app and write its asset bundle. |
| `topcoat asset clean` | Delete asset bundles and the asset download cache. |
| `topcoat ui init` | Create `components.toml` and install the theme stylesheet. |
| `topcoat ui add` | Copy registry components into the package. |
| `topcoat ui list` | List registry components and their install status. |
| `topcoat ui remove` | Delete installed components. |

There is no other subcommand in the 0.10.0 source (`topcoat-cli-0.10.0/src/lib.rs:30`).

### Build flags (`dev`, `asset list`, `asset bundle`)

| Flag | Meaning |
|---|---|
| `--bin <BIN>` | Build the named binary target. |
| `-p, --package <PACKAGE>` | Build the named package. |
| `-r, --release` | Build with the `release` profile. Conflicts with `--profile`. |
| `--profile <NAME>` | Build with the named cargo profile. |

The CLI runs `cargo build --message-format=json-diagnostic-rendered-ansi` with these arguments and takes the single executable (or `cdylib`/`dylib`) artifact from the JSON messages. Zero artifacts is an error ("must be a `bin`, `cdylib`, or `dylib` target"); several is an error that asks for `--bin`, `--package` or `[package] default-run`. Each profile keeps its own bundle, so bundle with the profile that will run.

### `topcoat dev`

Status: UNVERIFIED (read from the source and `--help`; `topcoat dev` was not run)
Source: `topcoat-cli-0.10.0/src/dev.rs`

- Starts a broadcast server for browsers (the first free port from 59039, 100 ports tried), builds, bundles assets after each successful build, and starts the executable. A rebuild that does not change the executable (same path, mtime and length) leaves the running process alone. A failed build keeps the previous process serving.
- Starts the app with `TOPCOAT_DEV_URL=<broadcast url>`, `HOST` and `PORT` set. The address is `HOST` (default `127.0.0.1`) and `PORT` (default 3000) from the CLI's own environment; if the port is taken, it tries the next ones and reports the choice.
- `[package] default-run` in `Cargo.toml` selects the binary when the package has several (new in 0.10); an explicit `--bin` wins.
- Watches the workspace root and the directory of every local package, recursively. It ignores paths under the cargo target directory, hidden path components, editor temp files (`*~`, `4913`, `#*#`, `___jb_`) and anything matched by `.gitignore` files and `.git/info/exclude`. A `Cargo.toml` change re-derives the watched set. Changes are debounced by 50 ms.
- Pressing `r` on an interactive terminal forces a rebuild and a restart. Ctrl-C stops the build and the app.
- Live page updates need the app's cooperation: `topcoat::dev::script()` in `<head>` (renders the `dev.js` tag only when `TOPCOAT_DEV_URL` is set) and `topcoat::dev::notify_ready(addr)` after binding (needs the `serve` feature). `web` does neither (`web/src/document.rs` renders no `dev::script()`, `web/src/main.rs` binds with `DASHBOARD_BIND_ADDR`/`[dashboard].bind_addr` and calls `topcoat::serve`), so under `topcoat dev -p web` the app would be built, bundled and started, but it would ignore the `HOST` and `PORT` it is given (it binds from config) and the browser would not auto-reload. Not run against `web` here.

### `topcoat fmt`

Status: VERIFIED (`--check`, in-place and `--rustfmt` runs in the probes)
Source: `topcoat-cli-0.10.0/src/fmt.rs`, `docs/fmt.md`

| Flag | Meaning |
|---|---|
| `[FILES]...` | Files, directories or globs. Directories are searched for `*.rs` recursively. With none, `**/*.rs` under the working directory, which includes `target/`. |
| `--check` | Report files that need formatting on stderr and exit 1 if any, or if any cannot be formatted; write nothing. New in 0.10. |
| `--stdin` | Read source from stdin and write the formatted source to stdout (with `--check`, only the status). |
| `--rustfmt` | Run `rustfmt` first, then format macro bodies. Both must succeed before a file is written. The edition must be set in `rustfmt.toml` (`edition = "2024"`). New in 0.10. |
| `--macros <LIST>` | Comma-separated subset of: `view`, `attributes`, `class`, `live`, `emit`, `font_face`, `font`, `fontsource_font_face`, `fontsource_font`, `mail`. Unknown names exit 1. |

Final line: `successfully formatted N files (M modified) in T`, or `checked N inputs (M need formatting), F failed in T`. Macro bodies are printed with a fixed margin of 89 columns and a 4-space indent (`topcoat-core-grammar-0.10.0/src/pretty/printer.rs:11`); the workspace `rustfmt.toml` (`max_width = 85`) is not read for them. A one-line `Ok(view! { ... })` between 86 and 89 columns therefore oscillates between `topcoat fmt` (keeps it on one line) and `rustfmt` (wraps it in `Ok(...)`); observed in the registry's `card.rs` (see [ui.md](ui.md#3-declare-the-components)).

### `topcoat asset`

Status: VERIFIED (`--help`, the source and the bundle `topcoat asset bundle -p web` produced in `target/debug/assets`; `clean` and `list` were not run)
Source: `topcoat-cli-0.10.0/src/asset.rs` and `asset/*.rs`

| Subcommand | Behavior |
|---|---|
| `list` | Builds, then prints every asset source found in the binary: the file path, or the URL for remote assets. |
| `bundle [-o, --out <DIR>]` | Builds, then writes the bundle (hashed files plus `manifest.toml`) to `<DIR>`, by default `assets/` beside the built executable (`<target>/<profile>/assets`). Remote assets are downloaded through the cache at `<target>/topcoat/cache/assets`. |
| `clean [-o, --out <DIR>]` | Removes `<DIR>`, or every `assets/` directory under the target directory that contains a `manifest.toml`, plus the asset cache. |

Observed bundle for `web` (`target/debug/assets/`): `manifest.toml`, `tailwind-<hash>.css`, `topcoat-<hash>.js` (the browser runtime), `loadout-editor-<hash>.js`, `pending-submit-<hash>.js`. The server refuses to start without a bundle next to the executable (`AssetBundle::load()`), and a bundle only matches the build it came from.

### `topcoat ui`

Status: VERIFIED (`init`, `add`, `add --all`, `list` and `remove` run in a sandbox package, `scratch/topcoat-kb/ui/cli-sandbox`)
Source: `topcoat-cli-0.10.0/src/ui/*.rs`, `topcoat-ui-0.10.0/src/manage/*.rs`. Behavior of `add`, `remove` and `list` is tabulated in [ui.md](ui.md#install-behavior-topcoat-ui).

| Subcommand | Flags |
|---|---|
| `init` | `-c, --components-dir <DIR>` (default `src/components`), `-t, --theme <NAME>` (prompts when several exist; `neutral` is the only one in 0.10), `-p, --package <SPEC>`. Fails when `components.toml` exists. Writes the theme to `<package root>/<theme file>` (`styles.css`) and prints the Tailwind `build.rs` hint and the Geist/`font-fontsource` hint. |
| `add [COMPONENTS]...` | `-a, --all`, `-r, --registry <CRATE>`, `-o, --overwrite`, `-p, --package <SPEC>`. Without `--registry` the built-in `topcoat` registry is preferred; another dependency registry asks for confirmation (needs a terminal). |
| `list` | `-r, --registry`, `-i, --installed`, `-p, --package`. Statuses: available (listed bare), `(installed)`, `(update available)`, `(orphaned)`. |
| `remove <COMPONENTS>...` | `-r, --registry`, `-p, --package`. |

All four locate the package from the working directory (`cargo locate-project`) or `-p`, and resolve registries with `cargo metadata`. The built-in registry is the crate `topcoat-ui-registry`, which must be in the dependency graph through topcoat's `ui` feature (`web` enables it in `[build-dependencies]`). A custom registry is a crate with `[package.metadata.topcoat-ui] registry = "<dir>"` and a `registry.toml`, and must be a direct dependency.

## Environment variables

Status: VERIFIED (source; `RUSTFLAGS`, `RUSTUP_TOOLCHAIN` and `CARGO_TARGET_DIR` reaching the inner build was observed with the installed 0.10.0 binary).

| Variable | By | Meaning |
|---|---|---|
| `TOPCOAT_NO_VERSION_CHECK` | CLI reads | Any value disables the lockfile version warning. |
| `HOST`, `PORT` | `topcoat dev` reads and sets for the app; `topcoat::start` reads | Bind address. Defaults `127.0.0.1` and `3000`. `topcoat::serve(listener, ...)` (used by `web`) ignores them. |
| `TOPCOAT_DEV_URL` | `topcoat dev` sets; `topcoat::dev::script` and `notify_ready` read | URL of the dev broadcast server. |
| `CARGO_TERM_PROGRESS_WHEN=always`, `CARGO_TERM_PROGRESS_WIDTH=80` | CLI sets for its inner `cargo build` | Progress counts for the spinner. |
| `OUT_DIR`, `CARGO_MANIFEST_DIR` | build scripts read (`topcoat-tailwind`, `topcoat-icon`) | Where output goes, and the default working directory and relative icon cache directory. `iconify_icon!` and `include!` read `OUT_DIR` at compile time. |
| `TAILWIND_CLI` (name chosen by the build script) | `web/build.rs` through `executable_env` | Path to a Tailwind CLI binary; skips the download. |
| `TMPDIR`, `TMP`, `TEMP` | Tailwind build sets for the Tailwind child | A private `tailwind-scratch-<pid>-<n>` directory under `OUT_DIR`. |

What the CLI strips: nothing in 0.10. Version 0.9's inner build removed every `CARGO*` variable and `RUSTC`, `RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER`, `RUSTUP_TOOLCHAIN` and `RUSTFLAGS` (`topcoat-cli-0.9.0/src/common/cargo/build.rs:100-116`); 0.10 removed that loop, and its release notes state that CLI builds preserve Cargo and Rust environment settings. `RUSTUP_TOOLCHAIN`, `RUSTFLAGS`, `CARGO_TARGET_DIR`, wrappers and `.cargo/config.toml` therefore reach `topcoat dev` and `topcoat asset bundle` unchanged.

## Output and cache directories

Status: VERIFIED for `<target>/<profile>/assets/`, `components.toml`, `styles.css` and the component files; the cache paths are read from `topcoat-core-0.10.0/src/cache.rs` and the Tailwind and Iconify sources.

| Path | Contents | Written by |
|---|---|---|
| `<target>/<profile>/assets/` | Asset bundle: hashed files and `manifest.toml` | `asset bundle`, `dev` |
| `<target>/topcoat/cache/assets/` | Downloaded remote assets (URL `asset!`s, `host: Asset` fonts) | the bundler |
| `<target>/topcoat/cache/tailwind/` | `tailwindcss-<version>-<platform>` (and a `.lock` file) | Tailwind build |
| `<target>/topcoat/cache/iconify/<set>.json` | Default Iconify download cache | `iconify::BuildConfig::stage()` |
| `$OUT_DIR/tailwind.css` | The compiled stylesheet (default output) | Tailwind build |
| `$OUT_DIR/tailwind-input.css` | `@import "tailwindcss";`, only when no `input` is set | Tailwind build |
| `$OUT_DIR/topcoat-icon-iconify/<set>.json` | Icon sets copied here for the macros to read | `iconify::BuildConfig::stage()` |
| `components.toml`, `styles.css`, `<components_dir>/*.rs` | UI install state, theme, components | `ui` |

`<target>` is resolved from `OUT_DIR` (`<target>[/<triple>]/<profile>/build/<pkg>-<hash>/out`). When `OUT_DIR` does not follow that layout the Tailwind binary is cached in `OUT_DIR` itself and the iconify cache falls back to `OUT_DIR/topcoat-icon-iconify`.

## Tailwind build integration

Source: `topcoat-tailwind-0.10.0/src/build/*.rs`, `docs/tailwind.md`. Status: VERIFIED (probes `scratch/topcoat-kb/ui/build.rs`, `web-build/build.rs`; `web/build.rs` in the repository).

Features: `tailwind` on the `[build-dependencies]` entry for `BuildConfig`. The runtime dependency needs it only for `tailwind::stylesheet!()`, which expands to `asset!(concat!(env!("OUT_DIR"), "/tailwind.css"))`; enabling it there compiles `topcoat-tailwind` with its default `build` feature (`ureq`, `sha2`, `thiserror`) into the app. `web` leaves it off and writes the `asset!` call itself (`web/src/document.rs:22`).

```rust
fn main() {
    if let Err(error) = topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .executable_env("TAILWIND_CLI")
        .render()
    {
        println!("cargo::error=tailwind failed: {error}");
    }
    if let Err(error) = topcoat::icon::iconify::BuildConfig::new()
        .cache_dir("icons")
        .icon_set("lucide")
        .stage()
    {
        println!("cargo::error=iconify failed: {error}");
    }
    println!("cargo::rerun-if-env-changed=TAILWIND_CLI");
    println!("cargo::rerun-if-changed=styles.css");
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=icons");
}
```

`BuildConfig` (builder; `render()` returns the output path):

| Method | Default | Effect |
|---|---|---|
| `executable_source(ExecutableSource)` | `Github { version: "4.3.2", checksum: None }` | Where the CLI comes from. |
| `version("4.3.2")` | | Download that GitHub release without verification. |
| `version_checksum("4.3.2", "sha256:<hex>")` | | Download and verify (only `sha256`); verification happens once, before the file enters the cache. |
| `executable("tailwindcss")` | | A bare name is resolved through `PATH`; a path is used as is, relative to the package root. No download. |
| `executable_env("TAILWIND_CLI")` | | Like `executable`, with the value read from the variable at build time. Unset is an error. |
| `input(path)` | generated `@import "tailwindcss";` | Input CSS; relative paths resolve against `cwd`. |
| `output(path)` | `$OUT_DIR/tailwind.css` | Output CSS. |
| `cwd(dir)` | `$CARGO_MANIFEST_DIR` | Directory scanned for class names and base for relative paths. |
| `optimize(bool)` | `false` | `--optimize`. |
| `minify(bool)` | `true` | `--minify`. |

The CLI is invoked as `tailwindcss -i <input> -o <output> --cwd <cwd> [--optimize] [--minify]`. Download (`ExecutableSource::Github`): `https://github.com/tailwindlabs/tailwindcss/releases/download/v<version>/<asset>` with asset names for macOS x64/arm64, Linux x64/arm64/armv7 and Windows x64/arm64; cached as `tailwindcss-<version>-<platform>` in `<target>/topcoat/cache/tailwind`, under a file lock; cached files are reused without verification. The pinned version for this repository is 4.3.2 (checksum for `tailwindcss-linux-x64`: `sha256:5036c4fb4328e0bcdbb6065c70d8ac9452e0d4c947113a788a8f94fd390425c1`).

Rebuild behavior: `render()` prints no `rerun-if-*`. Once a build script prints any, Cargo tracks only what it lists, so `web/build.rs` lists `TAILWIND_CLI`, `style`, `src` and `assets`. Do not point `rerun-if-changed` at a directory containing `target/`.

Scanning: Tailwind scans the working directory by default. `@import "tailwindcss" source(none);` plus `@source "./src/**/*.rs";` limits it to the Rust sources, as `web/style/input.css` does. The theme's `styles.css` adds the same `@source` but keeps the default auto-detection because it imports `tailwindcss` without `source(none)`.

## Font and icon modules

Status: VERIFIED (compiled and run in the probes), except the `host: Asset` download at bundle time: UNVERIFIED (the faces compile and the bundler's download code was read in `topcoat-asset-0.10.0/src/bundler/cache.rs`, but `topcoat asset bundle` was not run with it)
Features: `font`, `font-fontsource`, `icon`, `icon-iconify`
Source: `topcoat-font-0.10.0/src/{component,router}.rs`, `topcoat-font-0.10.0/src/fontsource/`, `topcoat-icon-0.10.0/src/iconify/config.rs`, `topcoat-icon-grammar-0.10.0/src/iconify/staged.rs`
Probe: `scratch/topcoat-kb/ui/tests/font_and_stylesheet.rs`, `scratch/topcoat-kb/ui/vendored/`, `scratch/topcoat-kb/ui/local-icons/`

| Module | Feature | What it adds | Build-time network | Run-time network |
|---|---|---|---|---|
| `topcoat::font` | `font` | `font!`, `font_face!`, `Font`, `link`, `preload_link`, `RouterBuilderFontExt::font` | none | the browser fetches whatever `src: url(...)` names |
| `topcoat::font::fontsource` | `font-fontsource` (implies `font`) | `fontsource_font!`, `fontsource_font_face!`, `families` (vendored catalog of 2088 families, generated by `topcoat-font`'s build script from `fonts.json`) | none | the browser fetches `https://cdn.jsdelivr.net/fontsource/fonts/<id>@latest/<subset>-<weight>-<style>.woff2` (default host) |
| `fontsource_font!(..., host: Asset)` | `font-fontsource` + `asset` | each face is an `asset!(url)` | none at compile time; `topcoat asset bundle` downloads each file through `ureq` into `<target>/topcoat/cache/assets` and the app serves them | none to third parties |
| `topcoat::icon` | `icon` (implies `view`) | `IconData`, the `icon` component | none | none |
| `topcoat::icon::iconify` | `icon-iconify` (implies `icon`) | `iconify_icon!`, `include!`, and, for build scripts, `BuildConfig` | the build script downloads `https://cdn.jsdelivr.net/npm/@iconify-json/<set>@<latest|version>/icons.json` for a set that is not cached | none |

Font arguments (`fontsource_font!`): the family first (the `SCREAMING_SNAKE_CASE` const in `families`), then `weight` (100..=900, one or a list), `style` (`Normal`, `Italic`), `subset` (default: the family's default subset), `host` (`JsDelivr` default or `Asset`) and `display` (`FontDisplay`, default `swap`). Each weight x style x subset is a separate face and, with `link`, a separate preload. Registration on the router is explicit: `.font(FONT)`; with the `discover` feature `font!` constants are also collected by `.discover()` (`web` does not enable `discover`). Served at `/_topcoat/fonts/<Family>-<hash>.css` with `cache-control: public, max-age=31536000, immutable`. Verified in `scratch/topcoat-kb/ui/tests/font_and_stylesheet.rs`.

Iconify (`iconify::BuildConfig`): `icon_set("lucide")`, `icon_set_version("mdi", "1.30.0")`, `cache_dir("icons")`, `stage()` (writes `$OUT_DIR/topcoat-icon-iconify/<set>.json`). A pinned version is re-downloaded when the sidecar `<set>.version` differs; an unpinned set is never refreshed (delete the cached file). A set in `cache_dir` is used as is, but its `prefix` must equal the set name and every alias must resolve. `iconify_icon!("set:name")` and `include!("set")` read the calling crate's `OUT_DIR`, so the crate that uses the macros must call `stage()` for the set in its own `build.rs`; without it the compile error includes the snippet to add. Verified: a committed one-file set builds with no network (`scratch/topcoat-kb/ui/vendored`, run in a network-less namespace).

### Dependency weight

Status: VERIFIED (`cargo tree -e normal` and `-e normal,build` in probe crates).

Measured with `cargo tree` for the feature sets `web` uses today plus each addition (`scratch/topcoat-kb/ui/deps/`; `web`'s `asset, compression, cookie, router, runtime, serve, view` and build-dependencies `tailwind, ui`):

| Addition | New crates (normal and build edges) | Notes |
|---|---|---|
| none | 170 crates (all edges) | baseline of that tree; `ureq`, `ureq-proto`, `rustls`, `rustls-webpki`, `webpki-roots`, `ring` are already present through `topcoat-tailwind`'s build feature |
| `font-fontsource` on the runtime dependency | 3: `topcoat-font`, `topcoat-font-macro`, `topcoat-font-grammar` | no third-party crates |
| `icon` | 3: `topcoat-icon`, `topcoat-icon-macro`, `topcoat-icon-grammar` | no third-party crates |
| `icon-iconify` on both the runtime and the build dependency | the same 3 | no further crate in the lock beyond those three, but at run time `icon-iconify` enables `topcoat-icon/iconify-build`, which links `ureq`, `rustls`, `rustls-webpki`, `webpki-roots`, `ring`, `base64`, `untrusted`, `utf8-zero`, `zeroize`, `ureq-proto` and `rustls-pki-types` into the application (14 more entries in the probe's normal-dependency tree, 153 to 167). In the real `web` tree (`cargo tree -p web -e normal`) only `ureq`, `ureq-proto` and `utf8-zero` are new; the TLS stack is already linked |

All six topcoat crates are MIT. The repository's `Cargo.lock` lists `topcoat-font*` but not `topcoat-icon*`; enabling `icon` adds the three icon crates to the lock.

Feature interplay observed while building the probes: topcoat 0.10.0 does not compile with `runtime` and without `asset` (`Asset: AttributeValueViewParts` is not satisfied in topcoat's own `runtime` module), so `runtime` needs `asset` next to it.

## How this repository builds `web`

Status: VERIFIED for the files read (`web/build.rs`, `web/Cargo.toml`, `docker/Dockerfile.web`, `.github/workflows/ci.yml`, `.dockerignore`, `README.md`); the Docker and CI jobs were not run. The proposed build script was run in `scratch/topcoat-kb/ui/web-build`.

### Locally

- `web/Cargo.toml`: `topcoat` (workspace entry `version = "0.10", default-features = false`) with `asset`, `compression`, `cookie`, `router`, `runtime`, `serve`, `view`; `[build-dependencies]` `topcoat` with `tailwind`, `ui`. No `font`, `icon` or `discover`.
- `web/build.rs`: `BuildConfig::new().input("style/input.css")`, then `executable_env("TAILWIND_CLI")` when `TAILWIND_CLI` is set and non-empty, else on Linux x86-64 `version_checksum("4.3.2", "sha256:5036...425c1")`, else `version("4.3.2")`. Errors are printed as `cargo::error=tailwind failed: ...`. It prints `rerun-if-env-changed=TAILWIND_CLI` and `rerun-if-changed` for `style`, `src` and `assets`.
- `web/src/document.rs`: `STYLESHEET = asset!(concat!(env!("OUT_DIR"), "/tailwind.css"))` linked in `<head>`, followed by `topcoat::runtime::script()` and the `pending-submit.js` asset.
- `web/src/main.rs`: `AssetBundle::load()` and `Router::builder().app_context(state).assets(assets)`, then `topcoat::serve(listener, router(base))`.
- Workflow (README, `bacon.toml` job `bundle-web`): `topcoat asset bundle -p web` after every change, then `cargo run -p web`; `topcoat fmt web/src web/tests && cargo fmt`.
- The toolchain is nightly through `rust-toolchain.toml`, and `.cargo/config.toml` sets `rustflags = ["-Z", "threads=8"]` and `RUST_TEST_THREADS=8`. 0.10's CLI passes them through (see [environment variables](#environment-variables)), which is the intended local behavior.

### Docker (`docker/Dockerfile.web`)

1. `chef`: `rust:1-slim-trixie`, `RUSTUP_TOOLCHAIN=stable`, `cargo install cargo-chef --locked`.
2. `planner`: `cargo chef prepare`.
3. `tailwind`: `debian:trixie-slim`; `curl` of `tailwindcss-linux-x64` v4.3.2 from GitHub and `sha256sum -c` against `5036c4fb...425c1`, so the build script never downloads an unverified binary.
4. `builder`: build tools and `libssl-dev`; `cargo install topcoat-cli@0.10.0 --locked`; the Tailwind binary copied to `/usr/local/bin/tailwindcss`; `ENV SQLX_OFFLINE=true TAILWIND_CLI=/usr/local/bin/tailwindcss`; `cargo chef cook --release -p web`; `COPY . .`; `rm -f rust-toolchain.toml && topcoat asset bundle --release -p web`.
5. `runtime`: copies `/app/target/release/web` and `/app/target/release/assets` side by side; runs as the `zayden` user on `0.0.0.0:3000`.

`.dockerignore` excludes `.cargo/`, `rust-toolchain.toml`, `*.md`, `design-docs/`, `docs/` and `scratch/`. Nothing a font or icon change adds under `web/` is excluded.

### CI (`.github/workflows/ci.yml`)

- Global env: `RUSTUP_TOOLCHAIN: stable`, `RUSTFLAGS: ""`, `SQLX_OFFLINE: true`, `TAILWIND_VERSION: 4.3.2`, `TAILWIND_SHA256`.
- Every job that compiles `web` reuses the `Fetch Tailwind CLI` step (a YAML anchor): download, `sha256sum -c`, `TAILWIND_CLI` exported through `GITHUB_ENV`.
- `web-bundle`: installs `topcoat-cli@0.10.0 --locked`, then `rm -f rust-toolchain.toml`, `rm -rf .cargo`, `topcoat asset bundle -p web`, and checks `target/debug/assets/manifest.toml` is non-empty. It runs on `ubuntu-latest` with network.
- `web-image`: `docker/build-push-action` builds `docker/Dockerfile.web` with a gha cache, without pushing.
- `deny`: `EmbarkStudios/cargo-deny-action@v2` `check`. The `fmt` job runs `cargo fmt --all --check` on nightly; `clippy` runs `cargo clippy --workspace --all-targets --locked -- -D warnings` on stable. `topcoat fmt --check` is not run in CI.

## What enabling fonts and icons changes

Status: VERIFIED for the lock, dependency and run-time rows; the Docker and CI rows follow from the files read above and were not run.

| Area | `font-fontsource` | `icon-iconify` | `icon` (local SVG) |
|---|---|---|---|
| `Cargo.lock` | no change (font crates are locked) | adds `topcoat-icon`, `-grammar`, `-macro` | same three |
| `--locked` CI | commit the lock | commit the lock | commit the lock |
| Build-time network | none | yes unless the set is vendored (`cache_dir("icons")` with a committed `lucide.json`): `latest` is fetched from jsDelivr with no checksum | none |
| Docker builder | none for the default `host`. With `host: Asset`, `topcoat asset bundle --release -p web` downloads the face files (the step has network). `fontsource_font!` passes no `checksum:` option to `asset!`. | the `web` build script runs during `topcoat asset bundle`; with a vendored set it needs nothing | none |
| CI `web-bundle` | none by default; `host: Asset` downloads in this step | as Docker | none |
| Offline / air-gapped builds | fine by default | vendor the set | fine |
| Reproducibility | the CSS pins no font version: files come from `@latest` on jsDelivr | `latest` unless pinned or vendored | exact |
| Run time | each visitor's browser requests the font files from `cdn.jsdelivr.net` (default host) and preloads every face; `host: Asset` serves them from `/_topcoat/assets/` | none | none |
| Binary | none | links `ureq`, `ureq-proto` and `utf8-zero` (the rest of the stack is already in `web`) | none |
| `cargo deny` | no new licenses (MIT) | the 3 new topcoat crates are MIT; `deny.toml` has `[graph] all-features = true` and the TLS crates are already in the graph through `topcoat-tailwind` | MIT only |
| `deny.toml` `[sources]` | no change: all crates come from crates.io | no change | no change |

Practical consequences for this repository: the Docker image already reaches the network at build time (rustup base image, `cargo install`, crate downloads, Tailwind download), so a default Iconify or `host: Asset` download works but is the only unpinned, unchecksummed fetch; vendoring the icon set and the font files (`font!` with `url(asset!("./fonts/<file>.woff2"))`) keeps the build deterministic and keeps the checksummed Tailwind download the only external fetch. Fonts served from jsDelivr are third-party requests from the dashboard; `host: Asset` or local files avoid them.

## Corrections to older notes

Status: VERIFIED (both corrections reproduced with the installed 0.10.0 binary).

- `README.md` says "`topcoat fmt` ... has no `--check` mode". It does in 0.10: `topcoat fmt --check [paths]`, exit status 1 when files need formatting.
- `README.md` says the CLI "clears `RUSTUP_TOOLCHAIN` and `RUSTFLAGS` for its inner build, so it always builds with the toolchain `rust-toolchain.toml` selects". That was 0.9 behavior; 0.10 passes the environment through. Locally the toolchain is still the one `rust-toolchain.toml` selects unless `RUSTUP_TOOLCHAIN` is exported. CI and Docker set `RUSTUP_TOOLCHAIN=stable` and `RUSTFLAGS=""` explicitly, which now takes effect directly.
