//! Every mutation gained an in-flight state so a double-click cannot fire it
//! twice while the first request is still on the wire. Nothing in the type
//! system enforces the shape of that guard: a new submit button can ship with
//! no `disabled=` binding at all, a page with several actions can disable the
//! wrong one after a copy-paste, the save button can grey out with no label
//! telling the reader why, and a `disabled` attribute with no matching
//! stylesheet rule leaves the busy state invisible either way.

use std::fs;
use std::path::{Path, PathBuf};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

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

fn sources(dir: &str) -> Vec<(String, String)> {
    files(dir, "rs")
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            let name = path.file_name()?.to_string_lossy().into_owned();
            Some((name, text))
        })
        .collect()
}

fn stylesheet() -> String {
    files("style", "css")
        .into_iter()
        .filter_map(|path| fs::read_to_string(&path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `<button ...>` opening tag, attributes only, paired with the file it
/// came from. Tags spanning several lines are fine: the split runs over the
/// whole file text, not a single line.
fn button_tags() -> Vec<(String, String)> {
    let mut tags = Vec::new();

    for (name, text) in sources("src") {
        for chunk in text.split("<button").skip(1) {
            let Some((attrs, _)) = chunk.split_once('>') else { continue };
            tags.push((name.clone(), attrs.to_owned()));
        }
    }

    tags
}

fn submit_buttons() -> Vec<(String, String)> {
    button_tags()
        .into_iter()
        .filter(|(_, attrs)| attrs.contains("type=\"submit\""))
        .collect()
}

fn class_tokens(attrs: &str) -> Vec<&str> {
    let Some((_, rest)) = attrs.split_once("class=\"") else { return Vec::new() };
    let Some((classes, _)) = rest.split_once('"') else { return Vec::new() };
    classes.split_whitespace().collect()
}

/// The name bound by `<ActionForm action=NAME`, if the line opens one.
fn action_name(line: &str) -> Option<&str> {
    let (_, rest) = line.split_once("<ActionForm action=")?;
    rest.split(|c: char| c.is_whitespace() || c == '>').next()
}

/// The action a line threads its in-flight state from, whether it binds the
/// attribute directly (`disabled=NAME.pending()`) or hands it to a component
/// that does (`pending=NAME.pending()`).
fn pending_binding(line: &str) -> Option<&str> {
    let (_, rest) =
        line.split_once("disabled=").or_else(|| line.split_once("pending="))?;
    let (name, _) = rest.split_once(".pending()")?;
    Some(name)
}

/// Every `NAME.pending()` found under `src/ui/pages`, paired with
/// whichever `<ActionForm action=..` most recently preceded it in the same
/// file, in source order.
fn pending_action_bindings() -> Vec<(String, Option<String>, String)> {
    let mut out = Vec::new();

    for (name, text) in sources("src/ui/pages") {
        let mut current: Option<String> = None;

        for line in text.lines() {
            if let Some(action) = action_name(line) {
                current = Some(action.to_owned());
            }

            if let Some(bound) = pending_binding(line) {
                out.push((name.clone(), current.clone(), bound.to_owned()));
            }
        }
    }

    out
}

/// A submit with no `disabled=` binding lets the reader double-click it and
/// fire the mutation twice.
#[test]
fn every_submit_button_declares_an_in_flight_state() {
    let offenders: Vec<_> = submit_buttons()
        .into_iter()
        .filter(|(_, attrs)| !attrs.contains("disabled="))
        .map(|(name, _)| name)
        .collect();

    assert!(offenders.is_empty(), "submit with no disabled= binding: {offenders:?}");
}

/// Several pages hold four or more actions on screen at once, so a
/// copy-pasted `disabled=` can end up disabling a neighbour's button instead
/// of its own.
#[test]
fn each_form_disables_its_own_actions_button() {
    let mismatches: Vec<_> = pending_action_bindings()
        .into_iter()
        .filter_map(|(file, action, bound)| {
            let action = action?;
            (action != bound).then(|| format!("{file}: {bound} under {action}"))
        })
        .collect();

    assert!(
        mismatches.is_empty(),
        "button disabled by the wrong action: {mismatches:?}"
    );
}

/// Disabling alone is a silent state; the label swap is the only thing that
/// says why the button stopped responding.
#[test]
fn the_save_button_says_when_it_is_busy() {
    const SETTINGS: &str = include_str!("../src/ui/components/settings.rs");
    const SAVING: &str = r#""Saving\u{2026}""#;

    assert!(SETTINGS.contains("disabled=pending"));
    assert!(SETTINGS.contains(SAVING), "save label does not swap while pending");
}

/// `disabled` with no rule is invisible: the button greys out nowhere and the
/// in-flight state is unreadable.
#[test]
fn every_disabled_control_has_a_disabled_rule() {
    let css = stylesheet();
    let offenders: Vec<_> = submit_buttons()
        .into_iter()
        .filter_map(|(name, attrs)| {
            let tokens = class_tokens(&attrs);
            let styled = tokens
                .iter()
                .any(|token| css.contains(format!(".{token}:disabled").as_str()));
            (!styled).then(|| format!("{name}: {tokens:?}"))
        })
        .collect();

    assert!(offenders.is_empty(), "disabled with no visible rule: {offenders:?}");
}

/// Every assertion above passes trivially against a scan that has quietly
/// stopped finding anything.
#[test]
fn the_scan_still_reaches_every_submit() {
    let rs_files = files("src", "rs");
    assert!(rs_files.len() > 40, "only {} .rs files scanned", rs_files.len());

    let submit = submit_buttons();
    assert!(submit.len() >= 13, "only {} submit buttons found", submit.len());

    let paired = pending_action_bindings()
        .into_iter()
        .filter(|(_, action, _)| action.is_some())
        .count();
    assert!(paired >= 30, "only {paired} pending bindings paired with an action");
}
