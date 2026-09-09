//! `<Suspense>`/`<Transition>` fallbacks used to reserve no space: either
//! `fallback=|| ()` (zero height) or a single italic "Loading…" paragraph
//! standing in for a whole card grid or multi-fieldset form. When the
//! boundary resolved, the page reflowed. Fallbacks were replaced with sized
//! `<Skeleton class="skeleton-…" count=N/>` placeholders shaped by rules in
//! `style/partials/skeleton.css`. These tests catch a new or edited boundary
//! regressing to a zero-height or unsized fallback, and catch a skeleton
//! shape class that gained no height rule in the stylesheet.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The three `fallback=|| ()` sites kept on purpose, by file. Two in
/// `layout.rs` (operator badge, operator sidebar link) resolve to nothing
/// for nearly every viewer, and login.rs must paint nothing until the
/// session resolves so an authenticated visitor never sees the sign-in
/// card flash before redirecting away.
const ALLOWLISTED_ZERO_HEIGHT: [(&str, usize); 2] =
    [("src/ui/components/layout.rs", 2), ("src/ui/pages/login.rs", 1)];

/// The three skeleton classes that are layout containers, not shapes: they
/// correctly have no height rule of their own.
const CONTAINERS: [&str; 3] = ["skeleton-list", "skeleton-grid", "skeleton-stack"];

const fn is_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_')
}

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

/// The stylesheet holding the skeleton shape rules, or empty text if it
/// cannot be read; the vacuity test below catches that case.
fn skeleton_css() -> String {
    let path = Path::new(CRATE_ROOT).join("style/partials/skeleton.css");
    fs::read_to_string(path).unwrap_or_default()
}

/// Every literal `class="…"` value that begins with `skeleton-`, across all
/// of `src/ui`. Derived from the markup rather than hard-coded so a new
/// shape class shows up here automatically.
fn skeleton_shape_classes() -> BTreeSet<String> {
    let mut shapes = BTreeSet::new();

    for (_, text) in ui_sources() {
        for attribute in text.split("class=\"").skip(1) {
            let Some((classes, _)) = attribute.split_once('"') else { continue };

            for class in classes.split_whitespace() {
                if class.starts_with("skeleton-") {
                    shapes.insert(class.to_owned());
                }
            }
        }
    }

    shapes
}

/// True if the stylesheet has some rule block whose selector list names
/// `shape` (as a whole class, not merely a prefix) and whose declaration
/// body sets `height`.
fn stylesheet_sizes_shape(css: &str, shape: &str) -> bool {
    let needle = format!(".{shape}");

    css.split('}').any(|chunk| {
        let Some((selectors, declarations)) = chunk.split_once('{') else {
            return false;
        };

        let names_shape = selectors
            .split(needle.as_str())
            .skip(1)
            .any(|rest| rest.chars().next().is_none_or(|c| !is_name(c)));

        names_shape && declarations.contains("height")
    })
}

/// A zero-height fallback reserves no space and reflows the page when the
/// boundary settles. That is correct only where the resolved content is
/// absent for nearly every viewer, or where nothing should paint at all;
/// anywhere else needs a sized skeleton. This test pins the short list so a
/// new zero-height fallback anywhere else fails loudly.
#[test]
fn zero_height_fallbacks_stay_on_the_short_list() {
    let expected: BTreeMap<String, usize> = ALLOWLISTED_ZERO_HEIGHT
        .iter()
        .map(|(path, count)| ((*path).to_string(), *count))
        .collect();

    let actual: BTreeMap<String, usize> = ui_sources()
        .into_iter()
        .filter_map(|(label, text)| {
            let count = text.matches("fallback=|| ()").count();
            (count > 0).then_some((label, count))
        })
        .collect();

    assert_eq!(
        actual, expected,
        "a zero-height fallback (`fallback=|| ()`) reserves no space and \
         reflows the page when the boundary settles; that is correct only \
         where the resolved content is absent for nearly every viewer or \
         where nothing should paint at all - anything else needs a sized \
         skeleton. Found: {actual:?}, expected: {expected:?}"
    );
}

/// The positive half of the rule above: a boundary that is not on the
/// zero-height allowlist must reserve space with a `Skeleton`. This catches
/// a new page shipping a one-line text fallback instead.
#[test]
fn every_boundary_file_reserves_space() {
    let allowlisted: Vec<&str> =
        ALLOWLISTED_ZERO_HEIGHT.iter().map(|(path, _)| *path).collect();

    let violations: Vec<_> = ui_sources()
        .into_iter()
        .filter(|(label, text)| {
            let has_boundary =
                text.contains("<Suspense") || text.contains("<Transition");
            has_boundary
                && !text.contains("Skeleton")
                && !allowlisted.contains(&label.as_str())
        })
        .map(|(label, _)| label)
        .collect();

    assert!(
        violations.is_empty(),
        "boundary file with no sized Skeleton fallback and not on the \
         zero-height allowlist: {violations:?}"
    );
}

/// A skeleton shape class with no height rule reserves nothing and is
/// `fallback=|| ()` in disguise.
#[test]
fn every_skeleton_shape_reserves_a_height() {
    let css = skeleton_css();

    let unsized_shapes: Vec<_> = skeleton_shape_classes()
        .into_iter()
        .filter(|class| !CONTAINERS.contains(&class.as_str()))
        .filter(|class| !stylesheet_sizes_shape(&css, class))
        .collect();

    assert!(
        unsized_shapes.is_empty(),
        "skeleton shape class with no `height` rule in skeleton.css - a \
         skeleton with no height reserves nothing and is `fallback=|| ()` \
         in disguise: {unsized_shapes:?}"
    );
}

/// Every assertion above passes trivially against a scan that has quietly
/// stopped finding anything.
#[test]
fn the_scan_still_reaches_every_boundary() {
    let rs_files = files("src", "rs");
    assert!(rs_files.len() > 40, "only {} .rs files scanned", rs_files.len());

    let ui = ui_sources();
    let skeleton_files =
        ui.iter().filter(|(_, text)| text.contains("class=\"skeleton")).count();
    assert!(
        skeleton_files >= 12,
        "only {skeleton_files} files under src/ui contain class=\"skeleton"
    );

    let shapes: Vec<_> = skeleton_shape_classes()
        .into_iter()
        .filter(|class| !CONTAINERS.contains(&class.as_str()))
        .collect();
    assert!(shapes.len() >= 4, "only {} distinct shape classes found", shapes.len());

    let css = skeleton_css();
    assert!(
        css.contains("@keyframes skeleton-pulse"),
        "skeleton.css lost its @keyframes skeleton-pulse rule"
    );
}
