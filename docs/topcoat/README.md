# Topcoat reference

Verified against: topcoat 0.11.0

Topcoat is pre-1.0 and its API changes between minor releases. This directory records what the pinned version actually provides, so code is written against checked facts instead of memory or search results. Read the relevant file before writing Topcoat code.

| File | Covers |
|---|---|
| [core.md](core.md) | Features, `view!`, components, client expressions, router, request context, forms, cookies, origin policy, assets, runtime script, head and meta, errors, test harness |
| [ui.md](ui.md) | Topcoat UI registry components, the `neutral` theme, Geist font, icons, wiring the installed components into `web` |
| [tooling.md](tooling.md) | `topcoat` CLI, Tailwind build integration, font and icon features, how `web` is built locally, in Docker and in CI |
| [changes-0.10-to-0.11.md](changes-0.10-to-0.11.md) | Release notes and public API differences between 0.10.0 and 0.11.0, with their impact on this repository |
| [changes-0.9-to-0.10.md](changes-0.9-to-0.10.md) | The same for 0.9.0 to 0.10.0 |

## Status labels

- **VERIFIED**: found in the pinned source and compiled in a probe crate.
- **UNVERIFIED**: documented upstream but not compiled here. Probe it before relying on it.
- **NO EQUIVALENT**: the feature does not exist in the pinned version. The entry names the fallback used here.

## Sources, in order of authority

1. The pinned crate sources (`~/.cargo/registry/src/*/topcoat-*-<version>/`) and docs.rs for that version.
2. Compiler output.
3. Observed runtime behavior.

An API that is not in (1) and has not compiled in (2) is treated as nonexistent.

`Probe:` lines name the local verification crates (under `scratch/topcoat-kb/`) the examples were compiled in. They are not committed. To re-verify an entry, paste its example into a crate that depends on `topcoat = "=<version>"` with the listed features and build it.

## Updating for a new Topcoat release

`web/tests/topcoat_reference.rs` fails when the `topcoat` version in `Cargo.lock` differs from the `Verified against` line above. When bumping Topcoat:

1. Read the release notes and diff the public API of the old and new crate sources.
2. Add `changes-<old>-to-<new>.md` in the format of the existing one, with the impact on this repository.
3. Re-check every entry in `core.md`, `ui.md` and `tooling.md` that the diff touches, recompile its example, and update its source citation.
4. Update the `Verified against` line in every file, then run `cargo test -p web --test topcoat_reference`.

## Updating vendored UI components

The files in `web/src/components/` that `web/components.toml` lists come from `topcoat-ui-registry`. The wired ones carry local edits (lint header, local icons, app tokens), so `topcoat ui add <name> --overwrite` would discard them.

1. Install the matching CLI (`cargo install topcoat-cli@<version> --locked`) and copy the wired files aside.
2. Run `topcoat ui add --all --overwrite -p web`. It rewrites every component, adds new ones and updates the hashes in `components.toml`.
3. Re-apply the local edits with a three-way merge per wired file: `git merge-file <saved copy> ~/.cargo/registry/src/*/topcoat-ui-registry-<old>/src/components/<name>.rs web/src/components/<name>.rs` writes the result into the saved copy, which then replaces the file. Resolve conflicts by keeping the upstream structure with the local class values.
4. `topcoat ui add` appends `pub mod` lines for every file it writes; restore `web/src/components/mod.rs` so only wired modules are declared.
5. Add new parked components to the `@source not` line of `web/styles.css`.

Parked components (listed in the `@source not` line of `web/styles.css`) stay byte-identical to upstream and can be overwritten freely.

## Wiring a parked component

1. Add `pub mod <name>;` to `web/src/components/mod.rs`.
2. Add the `#![expect(unreachable_pub, reason = "...")]` header used by the other wired components.
3. Replace each `iconify_icon!(...)` with a constant from `web/src/components/ui_icons.rs`, adding the constant there if it is missing. The `icon-iconify` feature stays off.
4. Remove the component from the `@source not` line in `web/styles.css`.
5. Map any registry token it uses to the `web/style` tokens.
