//! Source scans over the markup in `src` and the stylesheet partials: every
//! class has a rule, links to server routes opt out of client routing, every
//! POST form opts into the pending state, every submit button has a disabled
//! rule, every streamed boundary reserves space and the rail gives way to the
//! menu sheet below 1024 px.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

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

/// Component files installed from the Topcoat registry, as listed in
/// `components.toml`. They style themselves with the utility classes that
/// `styles.css` compiles, not the stylesheet partials, including after local
/// edits to the installed copy.
fn installed_components() -> BTreeSet<PathBuf> {
    let manifest = fs::read_to_string(Path::new(CRATE_ROOT).join("components.toml"))
        .unwrap_or_default();

    manifest
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("file = \"")?.strip_suffix('"'))
        .filter(|file| file.starts_with("src/"))
        .map(|file| Path::new(CRATE_ROOT).join(file))
        .collect()
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

/// The classes a component writes through `class=(name)` when the caller
/// passes nothing: the `#[default("…")] name` of the same source.
fn default_classes(source: &str, tag: &str) -> Vec<String> {
    let Some(name) = tag
        .split("class=(")
        .nth(1)
        .and_then(|rest| rest.split_once(')'))
        .map(|(name, _)| name)
    else {
        return Vec::new();
    };
    let source = squeezed(source);
    let parameter = format!(")] {name}:");

    source
        .split("#[default(\"")
        .skip(1)
        .filter_map(|rest| rest.split_once('"'))
        .find(|(_, after)| after.starts_with(&parameter))
        .map_or_else(Vec::new, |(classes, _)| {
            classes.split_whitespace().map(str::to_owned).collect()
        })
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
                .into_iter()
                .chain(default_classes(&source, &tag))
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
        checked >= 4,
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
        suspense_files().len() >= 4,
        "the scan stopped reaching the boundary files"
    );
}

#[test]
fn the_rail_gives_way_to_the_menu_button_below_1024() {
    let css = squeezed(&read("style/partials/chrome.css"));
    let (wide, narrow) = css
        .split_once("@media (max-width: 1023px) {")
        .expect("chrome.css has no tablet breakpoint");

    assert!(wide.contains(".menu-button { display: none; }"));
    assert!(wide.contains(".rail { position: fixed;"));
    assert!(narrow.contains(".rail { display: none; }"));
    assert!(narrow.contains(".menu-button { display: inline-flex; }"));
}

#[test]
fn the_rail_and_the_sheet_render_the_same_links_under_distinct_prefixes() {
    let rail = squeezed(&read("src/components/nav_rail.rs"));
    let sheet = squeezed(&read("src/components/nav_sheet.rs"));
    let links = squeezed(&read("src/components/nav_links.rs"));

    assert!(rail.contains(
        "nav_links(prefix: \"rail\", access: access, guild_id: guild_id)"
    ));
    assert!(sheet.contains(
        "nav_links(prefix: \"sheet\", access: access, guild_id: guild_id)"
    ));
    assert!(
        links.contains("for group in GROUPS"),
        "every group renders, none folds away"
    );
}
