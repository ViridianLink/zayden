# Build and toolchain post-mortems

## Build profiles

Both live in the root `Cargo.toml` and pull in opposite directions.

### `dev` — compile speed above all else

- `debug = false` is the big one. Backtraces lose line numbers and a debugger is
  effectively useless. To debug one crate, temporarily add
  `[profile.dev.package.<crate>] debug = true` rather than flipping it globally.
- `[profile.dev.build-override]` deliberately stays at `opt-level = 3`. Build
  scripts and proc-macros (serde, sqlx `query!`, serenity) compile once but
  _execute_ on every rebuild, so their runtime speed outweighs their build time.
- `.cargo/config.toml` adds `-Zthreads=8`. Returns from the parallel frontend
  flatten out past ~8 regardless of core count.

### `release` — smallest runtime memory footprint

The VPS bills on memory, so this trades CPU for resident size. Do not "fix" a
slow release build by loosening these; the cost is intentional.

Measured 2026-08-15 (32-core dev box, `cargo build --release --bin bot` from a
cold `target/`): **3m36s wall, 3.0 GB peak RSS, 23.6 MB stripped binary.** The
peak is the fat-LTO link step — comfortable on a 16 GB CI runner, so the LTO
settings are not a CI memory risk. If that ever changes, `lto = "thin"` is the
knob to reach for, not `codegen-units`.

These settings shrink the **text segment**, which is only part of resident size.
The larger lever on runtime RSS is the allocator — glibc's per-thread arenas
versus `MALLOC_ARENA_MAX=2`, or jemalloc with aggressive decay. Not done; it is
a code and deployment change rather than a profile one.

## Why there is no cranelift backend

The obvious next compile-speed win is `codegen-backend = "cranelift"`. It was
tried on 2026-08-15 and **does not work with this dependency tree.** Do not
re-add it without reading this.

Root cause: `aws-lc-sys` — pulled in by rustls, hence by serenity _and_ sqlx —
emits

```
cargo:rustc-link-lib=static:+whole-archive=aws_lc_0_44_0_crypto
```

and cg_clif loses that whole-archive native library. Three distinct failure
modes, each of which only appeared after the previous one was fixed:

1. Workspace binaries and test targets fail to link with a wall of
   `undefined symbol: aws_lc_0_44_0_*`.
2. Putting only the workspace members on LLVM (`[profile.dev.package."*"]` =
   cranelift, which matches dependencies but never workspace members) then moves
   the failure to proc-macro dylibs: `libsqlx_macros.so` fails to **load**. Note
   the `\x01` raw-symbol prefix in that error — it is a `dlopen` failure, not a
   link failure.
3. `[profile.dev.package.<name>]` takes precedence over
   `[profile.dev.build-override]`, so build-override cannot carve proc-macros
   back out. An explicit `package.sqlx-macros` opt-out still did not fix (2).

Two dead ends, so nobody re-runs them:

- **The linker is not the cause.** Fails identically under `rust-lld` (this
  nightly's default) and GNU `bfd`; succeeds under LLVM with either.
- **`aws-lc-sys`'s own backend is not the cause.** Overriding just that crate
  changes nothing — the final link is driven by the _leaf_ crate's backend.

Cranelift also has no wasm32 backend at all, so the dashboard's hydration bundle
would need its own LLVM profile even if the above were solved.

## The nightly/stable split

Local development, bacon, tests and `cargo-leptos` run on **nightly**, pinned by
`rust-toolchain.toml`. The Docker release images build on **stable**, set by
`ENV RUSTUP_TOOLCHAIN=stable` in both Dockerfiles. Two mechanisms keep that
working, and breaking either one breaks the release build:

1. **`Cargo.toml` must stay stable-parseable.** Nightly-only build settings live
   in `.cargo/config.toml` (currently just `-Zthreads`). Do **not** introduce
   `cargo-features = [...]` in `Cargo.toml` — stable cargo rejects that key
   outright and every Docker build fails immediately.
2. **`.dockerignore` excludes `.cargo/` and `rust-toolchain.toml`.** That is what
   keeps the nightly-only config out of the stable image. Remove either entry and
   the builds start failing on the `-Z` flags.

CI (`.github/workflows/ci.yml`) does the same thing with env vars rather than
`.dockerignore`: `RUSTUP_TOOLCHAIN=stable` outranks `rust-toolchain.toml`, and
`RUSTFLAGS=""` clears the `-Z` flags.

The `fmt` job is the one exception and overrides `RUSTUP_TOOLCHAIN` back to
nightly. Stable rustfmt ignores the unstable options in `rustfmt.toml` rather
than erroring, so it does not merely gate more weakly — it formats to a
*different* style (`imports_layout` and `imports_granularity` are the ones that
bite), and `--check` fails on correctly formatted code. The job compiles
nothing, so nightly there costs nothing.

The same split shows up in lints. Clippy's lint set differs between the two
channels, so a `#[expect(clippy::…)]` that is fulfilled on nightly can be
unfulfilled on stable and fail the `-D warnings` gate — `clippy::empty_enums` in
`dashboard/src/lib.rs` is `allow` rather than `expect` for exactly that reason.
The reverse case is only noise: `clippy::assert_is_empty` in the workspace lint
table does not exist on stable, and its `unknown_lints` warning is documented to
ignore `-D warnings`.
