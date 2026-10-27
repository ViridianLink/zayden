Verified against: topcoat 0.11.0 (Cargo.lock), 2026-10-26

# Topcoat 0.10.0 to 0.11.0

Sources: the [v0.11.0 release notes](https://github.com/tokio-rs/topcoat/releases/tag/v0.11.0), the per-crate `CHANGELOG.md` files, and a source diff of every published `topcoat*` crate between 0.10.0 and 0.11.0 (`topcoat-l10n`, `topcoat-l10n-macro` and `topcoat-l10n-grammar` are new). New behavior that `web` could rely on was compiled in the probes (`scratch/topcoat-kb/core/probe/tests/v011.rs`, `scratch/topcoat-kb/ui/tests/floating.rs`).

Impact column: **none** (the repo does not touch the changed API), **changed in web: `path`** (the repo was adjusted), **opportunity** (new API the app can use). Registry, icon, font, Tailwind and CLI entries are listed at the end and documented in [ui.md](ui.md) and [tooling.md](tooling.md).

## Breaking and behavior changes

| Change | 0.10.0 | 0.11.0 | Impact on this repo |
|---|---|---|---|
| Minimum Rust | 1.98 | 1.99 (`rust-version` in every crate) | changed in web: workspace `rust-version = "1.99"` (`Cargo.toml:8`); stable is 1.99.1, nightly unaffected |
| Rerendered inputs | a browser rerender copied `value`/`checked`/`selected` from the new markup, except the focused input's value | edited controls keep the user's input; untouched ones take the new value. `Expr::dynamic()` forces the value on every render (core.md section 4.4) | none: `web` has no connected page or shard; its `:value` bindings read signals (`web/src/admin/pages/servers.rs:117,129`) |
| Module discovery | a `module_router!()` registered every discovered handler in the binary | only handlers in its root module or below (`topcoat-router-0.11.0/src/module/router.rs:60-69`) | none: `discover` is off, routes are registered by name |
| Built-in server timeouts | Hyper ran without a timer, so its timeouts never fired | `TokioTimer` on HTTP/1 and HTTP/2; e.g. a connection that sends no request headers within 30 s is closed (`topcoat-router-0.11.0/src/service.rs:152-153`) | applies to `web` (`topcoat::serve`, `web/src/main.rs`); no code change |
| Cookie prefix adapters | `override_prefix_host().get("__Host-x")` looked up `__Host-__Host-x` | names may be bare or already prefixed (`topcoat-cookie-0.11.0/src/prefix.rs:113-123`) | none: `web` uses no prefix adapter |
| Asset URLs | the bundled filename was written into the URL as is | percent-encoded as one path segment, in rendered URLs and served routes (`topcoat-asset-0.11.0/src/catalog.rs:22-50`) | none: `web`'s asset names are plain |
| `Last-Event-ID` | `to_str()` (visible ASCII only) | decoded as UTF-8 | none (no SSE) |
| History navigation | a fast back/forward could show an older page after a newer one | stale navigations are cancelled (`topcoat-runtime-0.11.0/browser/src/render/navigation.ts:390-394`) | none (`web` does not use `link`) |
| `topcoat fmt` without paths | globbed `**/*.rs`, including `target/` | skips Git-ignored files and directories with `CACHEDIR.TAG` | none: `web` passes paths (`topcoat fmt web/src web/tests`) |
| `dropdown_menu` registry component | `<details>`/`<summary>` | native popover; `target`/`id` props are required (upgrade guide) | none: parked, not compiled |
| Registry theme shadows | `--shadow-xs`/`--shadow-sm` had values | `0 0 #0000`, values commented out | none for the theme (not used); see the `web` fix below |

## New APIs

| Area | API | Purpose | Impact on this repo |
|---|---|---|---|
| view | `element_id(cx) -> String` | per-instance, rerender-stable HTML IDs from component identity and call site (core.md section 3.5) | changed in web: `web/src/components/confirm_dialog.rs` (form fields keep their readable IDs) |
| runtime | `Expr::dynamic()` | evaluate an expression in the browser on every render even without signal reads | none |
| core | `impl From<&Error> for Error` | `?` on the `Result<&T, &Error>` of `#[memoize(as_ref)]` | none: `web` does not use `#[memoize]` |
| l10n | features `l10n`, `l10n-number`, `l10n-datetime`, `l10n-fluent`, `jiff`; `topcoat::l10n` (`Locale`, `negotiate`, `accept_language`, `locale(cx)`, `Number`, `DateTime`, `fluent::messages!` and `t!`) | localization (core.md section 1, UNVERIFIED beyond `cargo tree`) | none: not enabled |
| core | `ConstWriter::write_u32_le`, `ConstReader::read_u32_le` | internal cursor helpers | none |
| cli | `topcoat new`, `topcoat l10n check` | scaffold an app; check Fluent translations | none |

## Smaller source-level differences

| Crate | Change | Impact |
|---|---|---|
| `topcoat-view`, `-view-grammar`, `-view-macro` | balanced join tree for big views (no recursion-limit errors), component go-to-definition | none |
| `topcoat-view` | the changelog's "escape `<` in comment payloads" is not in `escape.rs` (`COMMENT_ESCAPES` unchanged); the fix is in the browser runtime, which now decodes comment-marker payloads by replacing `&amp;`, `&lt;`, `&gt;` and `&quot;` instead of parsing them as HTML (`topcoat-runtime-0.11.0/browser/src/dom/markers.ts:141-158`) | none |
| `topcoat-runtime` browser runtime | text reconciliation after content replacement, hydration of values containing `<`, navigation cancel | none |
| `topcoat-core` browser `morph.ts` | `syncProperties` removed; `preservedAttributes` only when `preserveFormState` | see "Rerendered inputs" |
| `topcoat-core-grammar` | path constants for `topcoat_l10n` | none |
| `topcoat-font*`, `topcoat-icon*`, `topcoat-mail`, `topcoat-session`, `topcoat-tailwind` | version bump only (Tailwind default still 4.3.2) | none |
| `topcoat` | `src/l10n.rs`, feature table, docs (`docs/l10n.md`) | none |

Unchanged and re-verified: `runtime` still fails to compile without `asset`; the string-literal handler defect (`<input id="lit"data-topcoat-on:click=...>`) is still present; `runtime::script()`, `link`, head handling and the test harness are unchanged.

## Release notes (v0.11.0), item by item

| Release-note item | Summary | Impact |
|---|---|---|
| Localization | `l10n` module, Fluent `messages!`/`t!`, `Number`, `DateTime`, `topcoat l10n check` | none (opportunity if the dashboard is translated) |
| Creating apps | `topcoat new` with `--recommended`/`--minimal` | none |
| Element IDs | `element_id(cx)` | opportunity |
| Popovers | `popover` component, `floating` helper, `side`/`align` on popover, dropdown menu, tooltip, hover card | see [ui.md](ui.md); parked in `web` |
| Navigation menu | `navigation_menu` component | parked in `web` |
| Edited form controls | rerenders keep edited input values; `Expr::dynamic` | none |
| Errors from memoized functions | `?` on `Result<T, &Error>` | none |
| New demo and examples | admin demo, Cloudflare Workers example | none |
| Documentation and fixes | see the tables above | none |
| Upgrade guide: Rust version | 1.99 | changed in web: `Cargo.toml:8` |
| Upgrade guide: dropdown menus | `element_id`, `target`/`id`/`close`, `group-has-[:popover-open]/dropdown-menu:` instead of `group-open:`, `align: Start` for the old alignment | none: parked |
| Upgrade guide: rerendered inputs | reset inputs through a signal or `Expr::dynamic` | none |
| Upgrade guide: formatting ignored files | pass ignored paths explicitly | none |

## Component registry, icons, fonts, Tailwind and CLI

| Crate | 0.10.0 to 0.11.0 | Where documented |
|---|---|---|
| `topcoat-ui-registry` | new `floating`, `popover`, `navigation_menu`; `dropdown_menu` rewritten on popovers; `tooltip` and `hover_card` anchor-positioned with `side`/`align`; `table` emits `data-slot="table"` and new padding/borders, `card` adapts to an edge table; `dialog`/`sheet` overlay `bg-black/50` without blur; class lists split over several `class!` strings; theme shadows off. `floating`'s `Right`/`Left` sides render a broken class (defect) | [ui.md](ui.md) |
| `topcoat-ui-registry` build | optional `embedded` feature (`FILES`), used by the CLI for `topcoat new` | [tooling.md](tooling.md) |
| `topcoat-icon`, `topcoat-font`, `topcoat-tailwind` | version bump only | [ui.md](ui.md), [tooling.md](tooling.md) |
| `topcoat-cli` | `new`, `l10n check`; `fmt` honors ignore rules; `dev` restarts on asset changes, notices new files, keeps hot reload after client navigation, runs translation checks; prompts via `cliclack` | [tooling.md](tooling.md) |

## Changes made in this repository

- `Cargo.toml`: `topcoat = { version = "0.11" }`, `rust-version = "1.99"`; `Cargo.lock` updated.
- `README.md`, `docker/Dockerfile.web`: `topcoat-cli@0.11.0`.
- `web/components.toml` and `web/src/components/*.rs`: `topcoat ui add --all --overwrite`; the 13 wired files were three-way merged so their local edits survive (upstream changes to them are formatting and doc comments); `floating`, `navigation_menu` and `popover` added as parked components (`web/styles.css` `@source not`).
- After the upgrade, `web` moved onto the registry and runtime APIs: registry `button_variants`, `badge_variants`, `input`, `textarea`, `select`, `alert`, `table`, `pagination` and `tabs` replace the app's own CSS-class markup ([ui.md](ui.md#what-is-installed-in-web)); internal links use `runtime::link_attrs` with prefetch off by default (core.md 12.2); routes use `module_router!` under `web/src/app/` (core.md 5.2).
- `web/style/partials/tokens.css`: `--shadow-xs` and `--shadow-sm` changed from `none` to `0 0 #0000`. With `none`, the combined `box-shadow` of `shadow-xs` plus `focus-visible:ring-2` was invalid and primary, secondary and destructive buttons lost their focus ring; the 0.11 theme documents the same trap.

## What to re-check when this file changes

- `core.md` sections 1 (features), 3.5 (`element_id`), 4.4 (bindings and rerenders), 5.2 (module discovery), 5.11 (serving), 6.3 and 15.1 (`?` on `&Error`), 8.2 (cookie prefixes), 11.1 (asset URLs).
- `ui.md` overlays (`dropdown_menu`, `popover`, `floating`, `tooltip`, `hover_card`), `navigation_menu`, `table`, `card`, theme shadows.
- The wired components in `web/src/components/` against the new registry sources.
