//! Every class in the markup needs a rule, or the control renders unstyled: the
//! Patreon tab's Disconnect button spent its life as an OS-grey browser default
//! because `danger` was defined nowhere. Nothing in the build joins the two
//! sides - Tailwind scans `src/` only for its own utilities and ignores names
//! it does not recognise - so this is the only check that can catch the drift.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

const fn is_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_')
}

/// Unreadable paths are skipped rather than reported; the vacuity test below is
/// what stops a scan that has quietly stopped finding anything.
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

/// Literal `class="…"` attributes only. A handful of sites build their class
/// from a signal instead; those are invisible here and stay the reviewer's job.
fn used_classes() -> BTreeMap<String, String> {
    let mut used = BTreeMap::new();

    for path in files("src", "rs") {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let file = path.file_name().unwrap_or_default().to_string_lossy();

        for attribute in text.split("class=\"").skip(1) {
            let Some((classes, _)) = attribute.split_once('"') else { continue };

            for class in classes.split_whitespace() {
                used.entry(class.to_owned()).or_insert_with(|| file.to_string());
            }
        }
    }

    used
}

fn defined_classes() -> BTreeSet<String> {
    let mut defined = BTreeSet::new();

    for path in files("style", "css") {
        let Ok(text) = fs::read_to_string(&path) else { continue };

        for selector in text.split('.').skip(1) {
            let class: String =
                selector.chars().take_while(|&c| is_name(c)).collect();

            // A length such as `0.6rem` is the only other dot in a stylesheet,
            // and no class name starts with a digit.
            if class.is_empty() || class.starts_with(|c: char| c.is_ascii_digit()) {
                continue;
            }

            defined.insert(class);
        }
    }

    defined
}

#[test]
fn every_class_in_the_markup_has_a_rule_in_the_stylesheet() {
    let defined = defined_classes();
    let orphans: Vec<_> = used_classes()
        .into_iter()
        .filter(|(class, _)| !defined.contains(class))
        .map(|(class, file)| format!("{class} ({file})"))
        .collect();

    assert!(orphans.is_empty(), "used in markup, styled nowhere: {orphans:?}");
}

/// A scanner that quietly stopped matching would keep the guard above green for
/// the rest of the project's life, so both sides have to show up in bulk.
#[test]
fn the_scan_still_reaches_both_sides() {
    assert!(used_classes().len() > 50, "the markup scan found almost nothing");
    assert!(
        defined_classes().len() > 50,
        "the stylesheet scan found almost nothing"
    );
}
