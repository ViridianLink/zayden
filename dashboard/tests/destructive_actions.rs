//! The confirmation and the feedback banner are markup-plus-stylesheet
//! contracts. Nothing in the type system notices when a destructive button is
//! added without a guard, when the `<details>` swap that turns the trigger into
//! Cancel is deleted from the partial, or when an alert loses the ARIA role
//! that decides whether a screen reader ever reads it out.

use std::fs;
use std::path::{Path, PathBuf};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const CONFIRM: &str = "confirm.rs";

/// Unreadable paths are skipped rather than reported; the vacuity test below is
/// what stops a scan that has quietly stopped finding anything.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|found| found == "rs") {
            out.push(path);
        }
    }
}

fn sources() -> Vec<(String, String)> {
    let mut paths = Vec::new();
    collect(&Path::new(CRATE_ROOT).join("src"), &mut paths);

    paths
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            let name = path.file_name()?.to_string_lossy().into_owned();
            Some((name, text))
        })
        .collect()
}

/// Every destructive control routes through `ConfirmButton`. A second file
/// reaching for `btn-danger` is a bare one-click delete, which is the defect.
#[test]
fn the_danger_style_is_only_reachable_through_the_confirm_component() {
    let unguarded: Vec<_> = sources()
        .into_iter()
        .filter(|(name, text)| name != CONFIRM && text.contains("btn-danger"))
        .map(|(name, _text)| name)
        .collect();

    assert!(unguarded.is_empty(), "danger buttons outside {CONFIRM}: {unguarded:?}");
}

/// The trigger is also the cancel: closed it reads "Delete", open it reads
/// "Cancel". Without both halves of the swap the summary shows the two labels
/// at once and there is no way out of the confirmation but to submit it.
#[test]
fn the_confirm_trigger_swaps_to_cancel_when_it_opens() {
    const CSS: &str = include_str!("../style/partials/confirm.css");

    assert!(CSS.contains(".confirm>summary .confirm-cancel"));
    assert!(CSS.contains(".confirm[open]>summary .confirm-label"));
    assert!(CSS.contains(".confirm[open]>summary .confirm-cancel"));
}

/// An alert nobody hears is the same defect as no alert, so every call site has
/// to choose between interrupting the reader and waiting its turn.
#[test]
fn every_alert_declares_how_loudly_it_announces_itself() {
    let mut silent = Vec::new();

    for (name, text) in sources() {
        for element in text.split("<Alert").skip(1) {
            let attributes = element.split("/>").next().unwrap_or_default();

            if !attributes.contains("role=") {
                silent.push(name.clone());
            }
        }
    }

    assert!(silent.is_empty(), "<Alert> without a role in: {silent:?}");
}

/// Dismissal is the whole point of the component: the banner it replaced sat
/// above the form until the next navigation, outliving the edit it described.
#[test]
fn the_alert_carries_a_labelled_dismiss_control() {
    const SETTINGS: &str = include_str!("../src/ui/components/settings.rs");

    assert!(SETTINGS.contains("class=\"alert-dismiss\""));
    assert!(SETTINGS.contains("aria-label=\"Dismiss\""));
}

/// Both scans above pass trivially against an empty file list.
#[test]
fn the_scan_still_reaches_the_markup() {
    let sources = sources();

    assert!(sources.len() > 40, "only {} sources scanned", sources.len());
    assert!(sources.iter().any(|(_name, text)| text.contains("<Alert")));
}
