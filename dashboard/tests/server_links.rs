//! Links to routes axum serves itself (OAuth hand-offs, login, invite) must
//! carry `rel="external"`. Without it `leptos_router` intercepts the click,
//! finds no matching page and renders the 404 view: the Patreon and YouTube
//! Connect buttons shipped that way and never reached their handlers.

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

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn is_server_path(path: &str) -> bool {
    SERVER_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

/// `let name = format!("/patreon/…")` bindings, so `href=name` resolves.
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

/// Every `<a …>` opening tag, attributes included, across line breaks.
fn anchors(source: &str) -> Vec<String> {
    source
        .split("<a ")
        .skip(1)
        .filter_map(|tail| tail.split_once('>'))
        .map(|(attrs, _)| {
            format!("<a {}>", attrs.split_whitespace().collect::<Vec<_>>().join(" "))
        })
        .collect()
}

fn targets_server(tag: &str, bindings: &[String]) -> bool {
    let literal = tag.split("href=\"").nth(1).is_some_and(is_server_path);
    let bound = bindings.iter().any(|name| {
        tag.contains(&format!("href={name} "))
            || tag.contains(&format!("href={name}>"))
    });

    literal || bound
}

#[test]
fn links_to_server_routes_bypass_the_client_router() {
    let mut files = Vec::new();
    collect(&Path::new(CRATE_ROOT).join("src/ui"), &mut files);

    let mut checked = 0_usize;
    let mut offenders = Vec::new();

    for file in files {
        let Ok(source) = fs::read_to_string(&file) else { continue };
        let bindings = server_bindings(&source);

        for tag in anchors(&source) {
            if !targets_server(&tag, &bindings) {
                continue;
            }

            checked += 1;
            if !tag.contains("rel=\"external") {
                offenders.push(format!("{}: {tag}", file.display()));
            }
        }
    }

    assert!(
        checked >= 4,
        "the scan found only {checked} server links; it has stopped matching"
    );
    assert!(
        offenders.is_empty(),
        "missing rel=\"external\":\n{}",
        offenders.join("\n")
    );
}
