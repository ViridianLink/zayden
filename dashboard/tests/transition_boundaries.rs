//! `<Suspense>` re-shows its `fallback` every time its resource returns to a
//! pending state, discarding the rendered children. `<Transition>` holds the
//! previously-rendered view instead. Several dashboard resources key on
//! `action.version()` signals, which change after every mutation, so a
//! `<Suspense>` boundary there collapses the whole form to a one-line
//! "Loading…" paragraph on every save. These tests catch that regression
//! reappearing on a new or edited settings tab.

use std::fs;
use std::path::{Path, PathBuf};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// Unreadable paths are skipped rather than reported; the vacuity test below
/// is what stops a scan that has quietly stopped finding anything.
fn collect(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, ext, out);
        } else if path.extension().is_some_and(|found| found == ext) {
            out.push(path);
        }
    }
}

fn files(dir: &str, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(&Path::new(CRATE_ROOT).join(dir), ext, &mut out);
    out
}

/// Every `.rs` file under `src/ui`, paired with its path relative to the
/// crate root so files sharing a name (several `mod.rs`) still name
/// themselves in a failure message.
fn ui_sources() -> Vec<(String, String)> {
    let root = Path::new(CRATE_ROOT);

    files("src/ui", "rs")
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            let rel = path.strip_prefix(root).unwrap_or(&path);
            Some((rel.to_string_lossy().into_owned(), text))
        })
        .collect()
}

/// A resource keyed on `action.version()` refetches after every mutation.
/// `<Suspense>` discards its children on every refetch, so pairing the two
/// greys a whole form out on every save; `<Transition>` is required there
/// instead.
#[test]
fn a_version_keyed_resource_never_sits_behind_a_suspense() {
    let offenders: Vec<_> = ui_sources()
        .into_iter()
        .filter(|(_, text)| {
            text.contains(".version()") && text.contains("<Suspense")
        })
        .map(|(label, _)| label)
        .collect();

    assert!(
        offenders.is_empty(),
        "version-keyed resource sits behind <Suspense>: {offenders:?}"
    );
}

/// The leaderboard's resource is keyed on the pagination and the "This
/// server"/"Global" toggle signals, both of which change on every
/// interaction; `<Suspense>` there would blank the table on every page turn.
#[test]
fn the_filter_keyed_leaderboard_holds_its_rows() {
    const LEVELS: &str = include_str!("../src/ui/pages/levels.rs");

    assert!(
        LEVELS.contains("<Transition"),
        "levels.rs lost its <Transition> boundary"
    );
    assert!(!LEVELS.contains("<Suspense"), "levels.rs regressed to <Suspense>");
}

/// The positive half of the rule above. Merely lacking `<Suspense>` is not
/// enough: a boundary deleted outright, or swapped for a `<Show>`, would
/// satisfy that test while reintroducing the same collapse.
#[test]
fn every_version_keyed_file_declares_a_transition() {
    let missing: Vec<_> = ui_sources()
        .into_iter()
        .filter(|(_, text)| {
            text.contains(".version()") && !text.contains("<Transition")
        })
        .map(|(label, _)| label)
        .collect();

    assert!(
        missing.is_empty(),
        "version-keyed resource with no <Transition> boundary: {missing:?}"
    );
}

/// Every assertion above passes trivially against a scan that has quietly
/// stopped finding anything.
#[test]
fn the_scan_still_reaches_every_boundary() {
    let rs_files = files("src", "rs");
    assert!(rs_files.len() > 40, "only {} .rs files scanned", rs_files.len());

    let ui = ui_sources();

    let transitions =
        ui.iter().filter(|(_, text)| text.contains("<Transition")).count();
    assert!(transitions >= 6, "only {transitions} files contain <Transition");

    let versioned =
        ui.iter().filter(|(_, text)| text.contains(".version()")).count();
    assert!(versioned >= 4, "only {versioned} files contain .version()");
}
