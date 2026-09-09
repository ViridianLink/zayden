//! UI-10: the login `Set-Cookie` omitted `max_age`, so a 7-day server
//! session was written to `web_sessions` while the browser held only a
//! *session* cookie it dropped on close. The fix made the omission
//! unrepresentable: `src/web/cookie.rs` owns a single `build` constructor
//! whose `max_age` parameter is required, and every cookie site now goes
//! through it. These tests scan source text rather than calling the code
//! because `src/web/`, `src/main.rs` and the cookie sites belong to the
//! `dashboard` binary crate, which an integration test in `tests/` cannot
//! see; only the library is reachable as Rust here.

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

/// Every `.rs` file under `src`, paired with its path relative to the crate
/// root so a failure names the offending file directly.
fn sources() -> Vec<(String, String)> {
    let root = Path::new(CRATE_ROOT);

    files("src", "rs")
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            let rel = path.strip_prefix(root).unwrap_or(&path);
            Some((rel.to_string_lossy().into_owned(), text))
        })
        .collect()
}

fn cookie_module_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/web/cookie.rs");
    fs::read_to_string(path).unwrap_or_default()
}

fn login_routes_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/web/routes_login.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// Every line number in `text` (1-indexed) where `needle` appears.
fn line_numbers_containing(text: &str, needle: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains(needle))
        .map(|(index, _)| index + 1)
        .collect()
}

/// A second call site building a `Cookie` directly bypasses the required
/// `max_age` parameter that made the original omission impossible - exactly
/// how UI-10 happened before the shared builder existed.
#[test]
fn every_cookie_goes_through_the_shared_builder() {
    let offenders: Vec<String> = sources()
        .into_iter()
        .filter(|(label, _)| label != "src/web/cookie.rs")
        .flat_map(|(label, text)| {
            line_numbers_containing(&text, "Cookie::build(")
                .into_iter()
                .map(move |line| format!("{label}:{line}"))
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "found `Cookie::build(` outside src/web/cookie.rs: {offenders:?} - \
         bypassing the shared builder is how `max_age` went missing in the \
         first place, silently downgrading a 7-day session cookie to one \
         the browser drops on close"
    );
}

/// Each attribute below is a security or correctness property the original
/// bug-fix pinned in the shared builder; a missing one regresses silently
/// for every caller at once.
#[test]
fn the_shared_builder_sets_every_security_attribute() {
    let text = cookie_module_source();

    let required = [
        ".path(\"/\")",
        ".http_only(true)",
        ".secure(",
        ".same_site(SameSite::Lax)",
        ".max_age(",
    ];

    let missing: Vec<&str> =
        required.into_iter().filter(|needle| !text.contains(needle)).collect();

    assert!(
        missing.is_empty(),
        "src/web/cookie.rs is missing attribute(s) from its shared builder: \
         {missing:?}"
    );

    assert!(
        text.contains("max_age: Duration"),
        "src/web/cookie.rs's `build` no longer takes `max_age: Duration` as \
         a required parameter - that is what makes omitting it impossible"
    );
}

/// The cookie's `max_age` and the database row's `expires_at` are both
/// derived from `SESSION_TTL_HOURS`, so the two cannot drift apart the way
/// they did before UI-10.
#[test]
fn the_session_cookie_and_its_row_share_one_ttl() {
    let text = login_routes_source();

    assert!(
        text.contains("SignedDuration::from_hours(SESSION_TTL_HOURS)"),
        "src/web/routes_login.rs no longer derives web_sessions.expires_at \
         from SignedDuration::from_hours(SESSION_TTL_HOURS)"
    );
    assert!(
        text.contains("Duration::hours(SESSION_TTL_HOURS)"),
        "src/web/routes_login.rs no longer derives the session cookie's \
         max_age from Duration::hours(SESSION_TTL_HOURS)"
    );
}

/// Every assertion above passes trivially against a scan that has quietly
/// stopped finding anything.
#[test]
fn the_scan_still_reaches_the_cookie_sites() {
    let rs_files = files("src", "rs");
    assert!(rs_files.len() > 40, "only {} .rs files scanned", rs_files.len());

    let cookie_module = cookie_module_source();
    assert!(!cookie_module.is_empty(), "src/web/cookie.rs read as empty");

    let call_sites: usize = sources()
        .iter()
        .map(|(_, text)| text.matches("cookie::build(").count())
        .sum();
    assert!(
        call_sites >= 4,
        "only {call_sites} occurrences of cookie::build( found across src/"
    );
}
