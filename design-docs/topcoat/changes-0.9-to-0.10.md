Verified against: topcoat 0.10.0 (Cargo.lock), 2026-10-06

# Topcoat 0.9.0 to 0.10.0

Sources: the [v0.10.0 release notes](https://github.com/tokio-rs/topcoat/releases/tag/v0.10.0), the per-crate `CHANGELOG.md` files, and a source diff of the published `topcoat`, `topcoat-view`, `topcoat-router`, `topcoat-runtime`, `topcoat-asset`, `topcoat-cookie` and `topcoat-core` crates (including their macro and grammar crates) between 0.9.0 and 0.10.0. The `topcoat` feature table itself is unchanged: every sub-crate dependency moved from 0.9.0 to 0.10.0 and nothing else in `Cargo.toml` differs.

Impact column: **none** (the repo does not touch the changed API), **changed in web: `path`** (the repo was adjusted), **opportunity** (new API the UI redesign can use). Entries that belong to the component registry, icons, fonts, Tailwind or the CLI are listed briefly at the end and documented in [ui.md](ui.md) and [tooling.md](tooling.md).

## Breaking and behavior changes

| Change | 0.9.0 | 0.10.0 | Impact on this repo |
|---|---|---|---|
| Runtime script tag | `<script type="module" src="..."></script>` | adds `data-topcoat-usize-bits="64"` (`usize::BITS`); hand-written tags pointing at `runtime::SCRIPT` are no longer sufficient, use `topcoat::runtime::script()` | changed in web: `web/tests/router.rs:139` asserts the new head markup. `web/src/document.rs:95` already uses `script()` |
| `String::len()` / `str::len()` inside `$(...)` and `expr!` | `f64` | `usize`; comparisons and arithmetic need `usize` operands (`name.get().len() > 100usize`) | none: no `$(...)` in `web/src` calls `len()` (the server filter uses `raw!` JavaScript, `web/src/admin/pages/servers.rs:77-100`) |
| `path_param!` | also emitted a `segment!` override, so it made the enclosing module a dynamic segment under `module_router!` | declares the parameter only; `module_param!` (new) declares it and sets the module segment. Under module routing a `path_param!` that relied on the segment now leaves the static segment and reading the parameter panics | none: `web` uses explicit absolute paths (`#[page("/guild/{guild_id}")]`) and does not use `module_router!` (`web/src/shell/guild_layout.rs:11`, `web/src/settings/mod.rs:30`, `web/src/admin/editor/mod.rs:15`) |
| Generated procedure and shard paths | random hex, different every build | `/_topcoat/runtime/procedures/<16 hex>` and `/_topcoat/runtime/shards/<16 hex>`, a hash of the function name and its source location (stable until renamed or moved) | none: `web` has no `#[procedure]` or `#[shard]` |
| `SuspenseMode::resolve(cx, explicit)` | associated function | removed; free function `topcoat::view::suspense_mode(cx)` (request context, then app context, then `Stream`); `suspense(.., mode: Option<SuspenseMode>)` unchanged | none: `web` uses plain `suspense(fallback:, child)` |
| Runtime WebSocket | one socket per connected page, shards separate | one socket per document carries every connected page and shard render; renders run side by side; at most 64 per socket (`429` beyond), configurable with `.max_runs_per_connection(n)`; the socket is requested at a page's URL only | none: `web` does not call `connected(cx)` |
| CLI build environment | inner `cargo build` removed every `CARGO*`, `RUSTC`, `RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER`, `RUSTUP_TOOLCHAIN` and `RUSTFLAGS` variable | environment is preserved, so `CARGO_TARGET_DIR`, `RUSTFLAGS`, wrappers and the toolchain from `rust-toolchain.toml` apply to `topcoat asset bundle` and `topcoat dev` | see [tooling.md](tooling.md); CI runs `cargo install topcoat-cli@0.10.0` then `topcoat asset bundle -p web` (`.github/workflows/ci.yml:158-163`) |

## New APIs

| Area | API | Purpose | Impact on this repo |
|---|---|---|---|
| runtime | `topcoat::runtime::link` (component), `link_attrs(cx, href, mode)`, `prefetch_mode(cx)`, `PrefetchMode::{Never, Intent, Viewport}`, `RouterBuilderRuntimeExt::prefetch(mode)` | navigate between server-rendered pages without a full document reload; updates the title and history, preserves signals declared on both pages, restores scroll; optional prefetch (core.md section 12.2) | opportunity: redesigned shell and sidebar navigation. Pages must be safe to render without being opened (prefetch). `web` links are plain `<a href>` today (`web/src/shell/sidebar.rs:94`; `web/src/components/sidebar.rs` is the uncompiled registry copy) |
| runtime | `#[record]` | named-field structs usable in signals, procedure arguments and expressions (struct literals, field access, `clone`) | opportunity: the loadout editor can hold structured state in a signal instead of a JavaScript module |
| runtime | tuple values in expressions: literals, `.0` access, nested tuples, `clone`; `TupleProject`, `TupleRefSurrogate` | tuples of vocabulary types (up to 12) in signals and `expr!` | opportunity |
| runtime | `Some(x)`, `Ok(x)`, `Err(x)` constructors in expressions now lower to `OptionSurrogate`/`ResultSurrogate` and take exactly one argument | | none |
| runtime | `#[diagnostic::on_unimplemented]` on `Surrogated`: ``'X' cannot be used in runtime expressions; declare a struct with #[record]`` | clearer errors for unsupported types | none |
| runtime | `RouterBuilderRuntimeExt::max_runs_per_connection(n)` | cap on concurrent connected renders per socket (default 64) | none |
| runtime | `expr!` keeps the user's tokens when lowering fails (`ExprInput`) | rust-analyzer completions keep working while an expression is incomplete | none (editor behavior) |
| view | `view!`/`live!`/`attributes!` forward incomplete Rust expressions and `let` bodies unchanged when they do not parse | same | none (editor behavior) |
| view | element-name binding identifiers are numbered per expansion instead of a global counter | deterministic macro output (`#485`) | none |
| view | `suspense_mode(cx)` free function | see above | none |
| router | `module_param!(name: T, error = ..)` and `module_param!(*name)` | declares a path parameter and sets the module's URL segment | none (module routing is not used) |
| router | `path_param!` accepts several declarations in one module; each is used in relative paths such as `#[page("./{post_id}")]` | | none |
| router | `docs/href.md` for `href!`; `module_router` docs renamed to `docs/module.md`; handler docs now use module routing | documentation | none |
| core | `impl Deref for Error { type Target = dyn std::error::Error + Send + Sync }` | borrow the stored error as a trait object | none |
| core | `thiserror` compatibility restored for `#[from] topcoat::Error` and `#[error(transparent)]` | `Error` can be a variant of an application error enum | none: `web` error enums do not wrap `topcoat::Error` (`web/src/auth/error.rs`) |
| core-grammar | `testing` feature and `testing` module | determinism tests for macro output | none |
| cli | `topcoat fmt --check` and `--rustfmt`, `--version` / `-V`, `topcoat dev` honors `[package] default-run`, `topcoat ui add --all [--overwrite]` | | see [tooling.md](tooling.md) and [ui.md](ui.md) |

## Smaller source-level differences

| Crate | Change | Impact |
|---|---|---|
| `topcoat-view` | `Attributes::iter` no longer `#[must_use]` | none |
| `topcoat-router` | `Next::run` no longer `#[must_use]` | none. `web/src/auth/middleware.rs:42` returns the result |
| `topcoat-router` | `SWAP_SCRIPT`, the one-time inline script that defines `window.topcoat.swap` for streamed `suspense` and `live!` regions, now ends with `document.currentScript.remove();` so it removes itself from the DOM | none (core.md section 14.1) |
| `topcoat-router` | doc examples for `#[page]`, `#[route]`, `StripPrefixLayer`, `Sitemap`, `Js`, `Wasm` switch to relative paths (`./sitemap.xml`) and module routing | none |
| `topcoat-runtime` | `RUNTIME_PROTOCOL` documentation: the subprotocol is requested at a page's URL, not at a shard's | none |
| `topcoat-runtime` browser runtime | new `navigation`, `prefetch`, `frames`, `record` and `tuple` modules; dev runtime event renamed `topcoat:dev-runtime:v2` | none |
| `topcoat-asset` | documentation only | none |
| `topcoat-cookie` | documentation and test assertions only (`#[route(POST)]` examples) | none |
| `topcoat-session` | version bump only | none (not enabled) |
| `topcoat` | `src/runtime.rs` (script tag), `src/view/suspense.rs` (`suspense_mode`), docs reorganized (`docs/context/` holds `app_context.md` and `functions_not_middlewares.md`; `alpine-ajax.md` became `alpine_ajax.md`) | see above |

## Release notes (v0.10.0), item by item

| Release-note item | Summary | Impact |
|---|---|---|
| Client side navigation | `runtime::link`, `link_attrs`, history and title updates, signal preservation, scroll restoration | opportunity |
| Prefetching | `PrefetchMode` per link, `.prefetch(..)` on the router builder, scoped `cx.with(PrefetchMode::Never)`; page rendering must be free of mutations | opportunity (audit pages that perform writes while rendering before enabling `Viewport`) |
| Records | `#[record]` and struct literals in expressions; every captured field, including private ones, is sent to the browser | opportunity |
| Tuple support | tuple literals, fields, nested tuples, `clone`; no tuple comparison | opportunity |
| Shard connections | one WebSocket per document; connected shards re-render without re-rendering a connected ancestor | none |
| Module parameters | `module_param!`; `path_param!` no longer changes the module segment. Upgrade: `module_param!` wherever `path_param!` made a module dynamic, including catch-alls | none |
| Bulk UI installation | `topcoat ui add --all [--registry r] [--overwrite]` | see [ui.md](ui.md) |
| rustfmt integration | `topcoat fmt --rustfmt` runs rustfmt first; both must succeed before a file is written | see [tooling.md](tooling.md) |
| Formatting checks | `topcoat fmt --check` reports differences, exit status 1 on differences or errors | see [tooling.md](tooling.md) |
| CLI version | `topcoat --version`, `cargo topcoat --version` | none |
| Default development binary | `topcoat dev` honors `[package] default-run`; `--bin` wins | none |
| Documentation and fixes | docs reorganized under `docs/`; `cargo topcoat dev` and `cargo topcoat fmt` work again; CLI builds preserve Cargo and Rust environment settings; dev reload applies streamed `live!` updates early; better rust-analyzer behavior inside macros; `topcoat::Error` works in `thiserror` variants again; macro output is deterministic (procedure and shard URLs use name and location); runtime WebSockets are limited to 64 simultaneous renders (`429`, `.max_runs_per_connection(..)`) | see the rows above |
| Upgrade guide: module routing parameters | see "Module parameters" | none |
| Upgrade guide: string lengths in expressions | `String::len()` and `str::len()` return `usize`; update comparisons (`> 100.0` becomes `> 100usize`); length still counts UTF-8 bytes | none |
| Upgrade guide: runtime script | `data-topcoat-usize-bits` is supplied by `runtime::script()`; replace hand-written tags that point at `runtime::SCRIPT` | changed in web: `web/tests/router.rs:139` |

## Component registry, icons, fonts, Tailwind and CLI

| Crate | 0.9.0 to 0.10.0 | Where documented |
|---|---|---|
| `topcoat-ui-registry` | components and themes unchanged (version bump only) | [ui.md](ui.md) |
| `topcoat-ui` / CLI `ui add` | `--all` flag installs every component of a registry, skipping existing files unless `--overwrite` | [ui.md](ui.md), [tooling.md](tooling.md) |
| `topcoat-icon`, `topcoat-font` | macro output made deterministic; no API change | [ui.md](ui.md), [tooling.md](tooling.md) |
| `topcoat-tailwind` | version bump only | [tooling.md](tooling.md) |
| `topcoat-cli` | `fmt --check`, `fmt --rustfmt`, `--version`, `default-run`, environment no longer stripped, `cargo topcoat` argument handling fixed, dev server reload fixes | [tooling.md](tooling.md) |
| `topcoat-htmx`, `topcoat-datastar`, `topcoat-mail` | version bump (mail: test assertions only) | not used |

## What to re-check when this file changes

- `core.md` sections 4 (client expressions), 5.2-5.3 (module routing, `path_param!`), 12 (script and `link`) and 14 (suspense).
- `web/tests/router.rs:127-147` (exact head markup), which pins the runtime script tag.
