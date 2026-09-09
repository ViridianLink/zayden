//! The dashboard stylesheet had no spacing scale: thirty-nine distinct
//! hand-tuned rem values scattered across `padding`/`margin`/`gap` in the
//! partials under `style/partials/`. A `--space-1`..`--space-10` ramp
//! replaced all of them, and four related duplications were collapsed at
//! the same time: an `--on-accent` token for ten hardcoded `#fff`s, one
//! shared panel shell for eleven copy-pasted card rules, one `.input`
//! primitive for three drifted form-control shells, and `.page` switched
//! from a hardcoded `960px` to `var(--content-max)`. Nothing else in the
//! build re-checks a stylesheet once it compiles, so these tests are what
//! stops any one of those five collapses regrowing silently, one component
//! at a time.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

const SPACING_PROPERTIES: [&str; 13] = [
    "padding",
    "margin",
    "gap",
    "row-gap",
    "column-gap",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
];

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

/// Every `.css` file under `style/partials`, paired with its path relative
/// to the crate root so a failure names the offending file directly.
fn partial_sources() -> Vec<(String, String)> {
    let root = Path::new(CRATE_ROOT);

    files("style/partials", "css")
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            let rel = path.strip_prefix(root).unwrap_or(&path);
            Some((rel.to_string_lossy().into_owned(), text))
        })
        .collect()
}

/// `tokens.css` is the ramp's definition, not a consumer of it; every scan
/// below except the vacuity canary excludes it.
fn partials_except_tokens() -> Vec<(String, String)> {
    partial_sources()
        .into_iter()
        .filter(|(label, _)| !label.ends_with("tokens.css"))
        .collect()
}

fn tokens_css() -> String {
    let path = Path::new(CRATE_ROOT).join("style/partials/tokens.css");
    fs::read_to_string(path).unwrap_or_default()
}

/// Removes every `var(...)` call from a declaration value so the ramp's own
/// `var(--space-N)` references never register as a bare rem literal.
fn strip_var_calls(value: &str) -> String {
    let mut result = String::new();
    let mut rest = value;

    while let Some((before, after)) = rest.split_once("var(") {
        result.push_str(before);
        rest = after.split_once(')').map_or("", |(_, tail)| tail);
    }

    result.push_str(rest);
    result
}

/// True if `value`, once `var(...)` calls are removed, still has a digit
/// directly followed by `rem` - a hand-tuned length rather than a ramp step.
fn has_bare_rem(value: &str) -> bool {
    let stripped = strip_var_calls(value);
    let parts: Vec<&str> = stripped.split("rem").collect();

    parts
        .iter()
        .take(parts.len().saturating_sub(1))
        .any(|part| part.ends_with(|c: char| c.is_ascii_digit()))
}

fn spacing_violations() -> Vec<String> {
    let mut violations = Vec::new();

    for (label, text) in partials_except_tokens() {
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            let Some(declaration) = trimmed.strip_suffix(';') else { continue };
            let Some((property, value)) = declaration.split_once(':') else {
                continue;
            };

            if !SPACING_PROPERTIES.contains(&property.trim()) {
                continue;
            }
            if !has_bare_rem(value) {
                continue;
            }

            violations.push(format!("{label}:{}: {trimmed}", index + 1));
        }
    }

    violations
}

/// `--space-N: <value>rem;` declarations pulled out of `tokens.css`, in
/// whatever order they appear.
fn ramp_steps() -> Vec<(u32, f64)> {
    let mut steps = Vec::new();

    for line in tokens_css().lines() {
        let Some(rest) = line.trim().strip_prefix("--space-") else { continue };
        let Some((number, value)) = rest.split_once(':') else { continue };
        let Ok(step) = number.trim().parse::<u32>() else { continue };
        let value = value.trim().trim_end_matches(';');
        let Some(rem) = value.strip_suffix("rem") else { continue };
        let Ok(rem) = rem.trim().parse::<f64>() else { continue };
        steps.push((step, rem));
    }

    steps
}

/// Every `var(--space-N)` reference across the partials, as (file, step).
fn space_references() -> Vec<(String, String)> {
    let mut refs = Vec::new();

    for (label, text) in partial_sources() {
        for chunk in text.split("var(--space-").skip(1) {
            let step: String =
                chunk.chars().take_while(char::is_ascii_digit).collect();
            if !step.is_empty() {
                refs.push((label.clone(), step));
            }
        }
    }

    refs
}

/// True if `text` contains a hex colour whose digits are exactly `fff` or
/// `ffffff`, case-insensitively - distinct from a colour that merely starts
/// with those digits, such as `#fffbeb`.
fn contains_hardcoded_white(text: &str) -> bool {
    text.split('#').skip(1).any(|chunk| {
        let hex: String =
            chunk.chars().take_while(char::is_ascii_hexdigit).collect();
        let hex = hex.to_lowercase();
        hex == "fff" || hex == "ffffff"
    })
}

/// Every rule block across all partials, as (file, selectors, declarations),
/// found by splitting on `}` then `{` - the same approach used for skeleton
/// shape rules in `skeleton_fallbacks.rs`.
fn rule_blocks() -> Vec<(String, String, String)> {
    let mut blocks = Vec::new();

    for (label, text) in partial_sources() {
        for chunk in text.split('}') {
            let Some((selectors, declarations)) = chunk.split_once('{') else {
                continue;
            };
            blocks.push((
                label.clone(),
                selectors.trim().to_owned(),
                declarations.to_owned(),
            ));
        }
    }

    blocks
}

/// The whole point of the ramp: a hand-tuned `1.25rem` reintroduces the
/// per-component drift the ramp replaced, one padding/margin/gap value at a
/// time, with nothing else in the build able to notice.
#[test]
fn every_spacing_value_comes_from_the_ramp() {
    let violations = spacing_violations();
    assert!(
        violations.is_empty(),
        "bare rem spacing value(s) outside the --space ramp:\n{}",
        violations.join("\n")
    );
}

/// A step deleted, renumbered or reordered either opens a gap a component
/// snaps past, or makes two components that meant to line up silently
/// diverge.
#[test]
fn the_ramp_is_contiguous_and_ascending() {
    let mut steps = ramp_steps();
    steps.sort_by_key(|(step, _)| *step);

    let numbers: Vec<u32> = steps.iter().map(|(step, _)| *step).collect();
    let expected: Vec<u32> = (1..=10).collect();
    assert_eq!(
        numbers, expected,
        "--space-N steps in tokens.css are not exactly 1..=10 with no gaps"
    );

    let values: Vec<f64> = steps.iter().map(|(_, value)| *value).collect();
    assert!(
        values.windows(2).all(|pair| pair[0] < pair[1]),
        "--space ramp values are not strictly increasing: {values:?}"
    );
}

/// `var(--space-11)` is a typo, not a bigger gap - it resolves to nothing,
/// which renders identically to no spacing at all.
#[test]
fn every_space_reference_resolves() {
    let defined: BTreeSet<u32> =
        ramp_steps().into_iter().map(|(step, _)| step).collect();

    let broken: Vec<String> = space_references()
        .into_iter()
        .filter_map(|(label, step)| {
            let parsed: u32 = step.parse().ok()?;
            (!defined.contains(&parsed))
                .then(|| format!("{label}: var(--space-{step})"))
        })
        .collect();

    assert!(
        broken.is_empty(),
        "var(--space-N) reference with no matching step in tokens.css: \
         {broken:?}"
    );
}

/// `--on-accent` replaced ten hardcoded `#fff`s standing in for the accent
/// contrast colour; a fresh one anywhere else is that duplication coming
/// back under a new selector.
#[test]
fn white_is_only_defined_once() {
    let offenders: Vec<String> = partials_except_tokens()
        .into_iter()
        .filter(|(_, text)| contains_hardcoded_white(text))
        .map(|(label, _)| label)
        .collect();

    assert!(
        offenders.is_empty(),
        "hardcoded white (#fff/#ffffff) found outside tokens.css, which \
         alone defines --on-accent: {offenders:?}"
    );
}

/// Eleven card-shaped classes shared one panel shell before this collapse;
/// a second block declaring the same three properties is that duplication
/// regrowing under a class the shared selector list does not name.
#[test]
fn the_panel_shell_is_declared_once() {
    let matches: Vec<String> = rule_blocks()
        .into_iter()
        .filter(|(_, _, declarations)| {
            declarations.contains("border-radius: var(--radius-2xl)")
                && declarations.contains("background-color: var(--bg-card)")
                && declarations.contains("border: 1px solid var(--border)")
        })
        .map(|(label, selectors, _)| format!("{label}: {selectors}"))
        .collect();

    assert!(
        matches.len() <= 1,
        "more than one rule block declares the full panel shell \
         (border-radius: var(--radius-2xl); background-color: \
         var(--bg-card); border: 1px solid var(--border)) - it should live \
         once, in card.css: {matches:?}"
    );
}

/// The `.input` shell in forms.css replaced three drifted copies of the
/// same form control; a second block matching both defining declarations
/// means a copy came back.
#[test]
fn the_form_control_shell_is_declared_once() {
    let matches: Vec<String> = rule_blocks()
        .into_iter()
        .filter(|(_, _, declarations)| {
            declarations.contains("background-color: var(--bg-base)")
                && declarations.contains("border-radius: var(--radius-xl)")
        })
        .map(|(label, selectors, _)| format!("{label}: {selectors}"))
        .collect();

    assert_eq!(
        matches.len(),
        1,
        "form control shell (background-color: var(--bg-base); \
         border-radius: var(--radius-xl)) should be declared exactly once, \
         by .input in forms.css: {matches:?}"
    );
}

/// `.page`'s content column moved from a hardcoded `960px` to
/// `var(--content-max)`; a literal max-width creeping back in decouples it
/// from every other consumer of that token.
#[test]
fn the_content_column_uses_its_token() {
    let path = Path::new(CRATE_ROOT).join("style/partials/pages.css");
    let css = fs::read_to_string(path).unwrap_or_default();

    assert!(
        css.contains("max-width: var(--content-max)"),
        "pages.css lost `max-width: var(--content-max)` on .page"
    );
    assert!(
        !css.contains("max-width: 960px"),
        "pages.css has a hardcoded max-width: 960px again"
    );
}

/// Every assertion above passes trivially against a scan that has quietly
/// stopped finding anything.
#[test]
fn the_scan_still_reaches_every_partial() {
    let partials = files("style/partials", "css");
    assert!(
        partials.len() >= 20,
        "only {} .css files scanned under style/partials",
        partials.len()
    );

    let space_refs = space_references().len();
    assert!(
        space_refs >= 160,
        "only {space_refs} var(--space- references seen across the partials"
    );

    assert!(tokens_css().contains("--on-accent"), "tokens.css lost --on-accent");
}
