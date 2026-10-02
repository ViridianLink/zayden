//! A cookie built outside the shared builder can omit `Max-Age`, which once
//! downgraded the 7-day session cookie to one the browser dropped on close.

use std::fs;
use std::path::{Path, PathBuf};

use topcoat::cookie::time::Duration;
use web::auth::build_cookie;

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const BUILDER_FILE: &str = "src/auth/cookie.rs";

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

/// Every `.rs` file under `src`, with its path relative to the crate root.
fn sources() -> Vec<(String, String)> {
    let root = Path::new(CRATE_ROOT);
    let mut files = Vec::new();
    collect(&root.join("src"), &mut files);

    files
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            let rel = path.strip_prefix(root).unwrap_or(&path);
            Some((rel.to_string_lossy().into_owned(), text))
        })
        .collect()
}

/// Ways to write a cookie without the shared builder's required `Max-Age`.
const BYPASSES: [&str; 4] = ["Cookie::build(", "Cookie::new(", "cookie!", ".add(("];

/// Receivers that write to the request's cookie jar.
const JAR_WRITES: [&str; 3] =
    ["jar.add(", "cookies(cx).add(", "cookie_jar(cx)?.add("];

/// Every jar write in `text` whose argument is not the shared builder.
fn unbuilt_writes(text: &str) -> usize {
    JAR_WRITES
        .iter()
        .flat_map(|write| {
            text.match_indices(write).map(move |(at, _)| at + write.len())
        })
        .filter(|&at| {
            let argument = text.get(at..).unwrap_or_default().trim_start();
            !argument.starts_with("cookie::build(")
                && !argument.starts_with("build_cookie(")
        })
        .count()
}

#[test]
fn every_cookie_goes_through_the_shared_builder() {
    let offenders: Vec<String> = sources()
        .into_iter()
        .filter(|(label, _)| label != BUILDER_FILE)
        .filter(|(_, text)| {
            BYPASSES.iter().any(|needle| text.contains(needle))
                || unbuilt_writes(text) > 0
        })
        .map(|(label, _)| label)
        .collect();

    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "cookies written outside {BUILDER_FILE}'s builder"
    );
}

#[test]
fn the_scan_detects_a_bypass() {
    assert_eq!(unbuilt_writes("cookies(cx).add((\"theme\", \"dark\"));"), 1);
    assert_eq!(
        unbuilt_writes("jar.add(\n    cookie::build(SESSION_COOKIE, v, d),\n);"),
        0
    );
}

/// The scan must keep reaching the crate's sources and the three cookie
/// writes in the sign-in routes (state, session, sign-out).
#[test]
fn the_scan_still_reaches_the_cookie_sites() {
    let sources = sources();
    assert!(sources.len() > 20, "only {} .rs files scanned", sources.len());
    assert!(sources.iter().any(|(label, _)| label == BUILDER_FILE));

    let call_sites: usize =
        sources.iter().map(|(_, text)| text.matches("cookie::build(").count()).sum();
    assert!(call_sites >= 3, "only {call_sites} uses of cookie::build( found");
}

#[test]
fn the_shared_builder_sets_every_attribute() {
    let cookie = build_cookie("name", "value", Duration::hours(1)).to_string();

    let expected = if cfg!(debug_assertions) {
        "name=value; HttpOnly; SameSite=Lax; Path=/; Max-Age=3600"
    } else {
        "name=value; HttpOnly; SameSite=Lax; Secure; Path=/; Max-Age=3600"
    };
    assert_eq!(cookie, expected);
}
