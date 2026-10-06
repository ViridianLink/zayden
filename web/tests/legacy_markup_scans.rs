//! Source scans over the markup in `src` and the stylesheet partials: every
//! class has a rule, links to server routes opt out of client routing, every
//! POST form opts into the pending state, every submit button has a disabled
//! rule, every streamed boundary reserves space and the sidebar module list
//! folds by class.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

const SERVER_PREFIXES: [&str; 7] = [
    "/auth/",
    "/invite",
    "/logout",
    "/kofi/",
    "/patreon/",
    "/youtube/",
    "/webhooks/",
];

const FORMS_WITHOUT_PENDING: [&str; 1] = ["src/shell/card.rs"];

const SKELETON_CONTAINERS: [&str; 3] =
    ["skeleton-list", "skeleton-grid", "skeleton-stack"];

const fn is_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_')
}

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

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Component files installed from the Topcoat registry and still identical to
/// the installed copy. They style themselves with utility classes, not the
/// stylesheet partials. A hand-written file that shares a registry name keeps
/// a different hash and stays scanned.
fn installed_components() -> BTreeSet<PathBuf> {
    let manifest = fs::read_to_string(Path::new(CRATE_ROOT).join("components.toml"))
        .unwrap_or_default();

    let mut hash = None;
    let mut out = BTreeSet::new();
    for line in manifest.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("hash = \"sha256:") {
            hash = rest.strip_suffix('"');
        } else if let Some(rest) = line.strip_prefix("file = \"") {
            let Some(file) =
                rest.strip_suffix('"').filter(|file| file.starts_with("src/"))
            else {
                continue;
            };
            let path = Path::new(CRATE_ROOT).join(file);
            let pristine = fs::read(&path).is_ok_and(|bytes| {
                hash.is_some_and(|expected| sha256_hex(&bytes) == expected)
            });
            if pristine {
                out.insert(path);
            }
        }
    }
    out
}

/// Every hand-written `.rs` file under `src`, as (path relative to the crate
/// root, text).
fn sources() -> Vec<(String, String)> {
    let root = Path::new(CRATE_ROOT);
    let installed = installed_components();

    files("src", "rs")
        .into_iter()
        .filter(|path| !installed.contains(path))
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            let rel = path.strip_prefix(root).unwrap_or(&path);
            Some((rel.to_string_lossy().into_owned(), text))
        })
        .collect()
}

fn stylesheet_sources() -> Vec<String> {
    files("style", "css")
        .into_iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect()
}

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(CRATE_ROOT).join(relative)).unwrap_or_default()
}

/// Collapses every run of whitespace to a single space so line wrapping
/// cannot break a multi-token match.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every opening tag named `name`, attributes included, across line breaks.
fn tags(source: &str, name: &str) -> Vec<String> {
    let open = format!("<{name}");

    source
        .split(open.as_str())
        .skip(1)
        .filter(|tail| tail.starts_with(char::is_whitespace))
        .filter_map(|tail| tail.split_once('>'))
        .map(|(attrs, _)| format!("{open} {}>", squeezed(attrs)))
        .collect()
}

fn literal_classes(tag: &str) -> Vec<String> {
    tag.split("class=\"")
        .nth(1)
        .and_then(|rest| rest.split_once('"'))
        .map_or_else(Vec::new, |(classes, _)| {
            classes.split_whitespace().map(str::to_owned).collect()
        })
}

/// Literal `class="…"` attributes only; classes chosen at run time are
/// invisible here.
fn used_classes() -> BTreeMap<String, String> {
    let mut used = BTreeMap::new();

    for (file, text) in sources() {
        for attribute in text.split("class=\"").skip(1) {
            let Some((classes, _)) = attribute.split_once('"') else { continue };

            for class in classes.split_whitespace() {
                used.entry(class.to_owned()).or_insert_with(|| file.clone());
            }
        }
    }

    used
}

fn defined_classes() -> BTreeSet<String> {
    let mut defined = BTreeSet::new();

    for text in stylesheet_sources() {
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

#[test]
fn the_class_scan_still_reaches_both_sides() {
    assert!(sources().len() > 40, "the markup scan found almost nothing");
    assert!(used_classes().len() > 50, "the markup scan found almost no classes");
    assert!(
        defined_classes().len() > 50,
        "the stylesheet scan found almost no classes"
    );
}

fn is_server_path(path: &str) -> bool {
    SERVER_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

/// `let name = format!("/patreon/…")` bindings, so `href=(name.as_str())`
/// resolves.
fn server_bindings(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("let ")?;
            let (name, value) = rest.split_once('=')?;
            let path = value.trim().strip_prefix("format!(\"")?;
            is_server_path(path).then(|| name.trim().to_owned())
        })
        .collect()
}

fn targets_server(tag: &str, bindings: &[String]) -> bool {
    let literal = tag.split("href=\"").nth(1).is_some_and(is_server_path);
    let bound = bindings.iter().any(|name| {
        tag.contains(&format!("href=({name})"))
            || tag.contains(&format!("href=({name}.as_str())"))
    });

    literal || bound
}

#[test]
fn links_to_server_routes_opt_out_of_client_routing() {
    let mut checked = 0_usize;
    let mut offenders = Vec::new();

    for (file, source) in sources() {
        let bindings = server_bindings(&source);

        for tag in tags(&source, "a") {
            if !targets_server(&tag, &bindings) {
                continue;
            }

            checked += 1;
            if !tag.contains("rel=\"external\"") {
                offenders.push(format!("{file}: {tag}"));
            }
        }
    }

    assert!(
        checked >= 8,
        "the scan found only {checked} server links; it has stopped matching"
    );
    assert!(
        offenders.is_empty(),
        "missing rel=\"external\":\n{}",
        offenders.join("\n")
    );
}

#[test]
fn every_post_form_opts_into_the_pending_state() {
    let mut checked = 0_usize;
    let mut offenders = Vec::new();

    for (file, source) in sources() {
        if FORMS_WITHOUT_PENDING.contains(&file.as_str()) {
            continue;
        }

        for tag in tags(&source, "form") {
            if !tag.contains("method=\"post\"") {
                continue;
            }

            checked += 1;
            if !tag.contains("data-pending") {
                offenders.push(format!("{file}: {tag}"));
            }
        }
    }

    assert!(
        checked >= 30,
        "only {checked} POST forms found; the scan has stopped matching"
    );
    assert!(
        offenders.is_empty(),
        "POST forms without data-pending:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn every_submit_button_has_a_disabled_rule() {
    let css = stylesheet_sources().join("\n");
    let mut checked = 0_usize;
    let mut offenders = Vec::new();

    for (file, source) in sources() {
        for tag in tags(&source, "button") {
            if !tag.contains("type=\"submit\"") {
                continue;
            }

            checked += 1;
            let styled = literal_classes(&tag)
                .iter()
                .any(|class| css.contains(&format!(".{class}:disabled")));
            if !styled {
                offenders.push(format!("{file}: {tag}"));
            }
        }
    }

    assert!(
        checked >= 10,
        "only {checked} submit buttons found; the scan has stopped matching"
    );
    assert!(
        offenders.is_empty(),
        "submit buttons with no :disabled rule:\n{}",
        offenders.join("\n")
    );
}

fn suspense_files() -> Vec<(String, String)> {
    sources().into_iter().filter(|(_, text)| text.contains("suspense(")).collect()
}

#[test]
fn every_suspense_fallback_reserves_space() {
    let mut checked = 0_usize;
    let mut offenders = Vec::new();

    for (file, text) in suspense_files() {
        for call in text.split("suspense(").skip(1) {
            checked += 1;
            let call = squeezed(call);
            let fallback = call.split_once("fallback:").map_or("", |(_, rest)| {
                rest.get(..rest.len().min(200)).unwrap_or(rest)
            });

            if !fallback.contains("skeleton") {
                offenders.push(format!(
                    "{file}: {}",
                    call.chars().take(80).collect::<String>()
                ));
            }
        }
    }

    assert!(
        checked >= 6,
        "only {checked} suspense calls found; the scan has stopped matching"
    );
    assert!(
        offenders.is_empty(),
        "suspense fallbacks that reserve no space:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn every_boundary_file_uses_the_skeleton_component() {
    let offenders: Vec<_> = suspense_files()
        .into_iter()
        .filter(|(_, text)| !text.contains("skeleton("))
        .map(|(file, _)| file)
        .collect();

    assert!(offenders.is_empty(), "boundaries without a skeleton: {offenders:?}");
}

#[test]
fn every_skeleton_container_class_in_the_markup_is_styled() {
    let css = read("style/partials/skeleton.css");
    let used = used_classes();

    for class in SKELETON_CONTAINERS {
        if used.contains_key(class) {
            assert!(css.contains(&format!(".{class}")), "{class} has no rule");
        }
    }
    assert!(css.contains("@keyframes skeleton-pulse"));
    assert!(
        suspense_files().len() >= 6,
        "the scan stopped reaching the boundary files"
    );
}

#[test]
fn the_caret_toggle_keeps_the_links_mounted() {
    let source = squeezed(&read("src/shell/sidebar.rs"));

    let open = source.find("\"app-sidebar-sublist open\"");
    let list = source.find("for module in MODULES");
    let (Some(open), Some(list)) = (open, list) else {
        panic!(
            "the module list or its open class is missing from src/shell/sidebar.rs"
        );
    };

    assert!(open < list, "the list must follow its class binding");
    let between = source.get(open..list).unwrap_or_default();
    assert!(
        !between.contains(" if ") && !between.contains(".then("),
        "toggling the caret must change a class, not rebuild the links: {between}"
    );
    assert!(source.contains("\"app-sidebar-sublist\""));
}

#[test]
fn the_collapsed_sublist_is_hidden_by_css() {
    let css = squeezed(&read("style/partials/layout.css"));

    assert!(css.contains(".app-sidebar-sublist { display: none;"));
    assert!(css.contains(".app-sidebar-sublist.open { display: flex;"));
}
