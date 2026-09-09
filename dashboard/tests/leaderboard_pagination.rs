//! UI-11 "Pagination dead-end": the levels leaderboard used to infer
//! `has_next` in the UI as `entries.len() == PAGE_SIZE`, so a board whose
//! size was an exact multiple of 10 left "Next" enabled onto an empty page,
//! and the empty-page branch rendered no pager at all, stranding the user.
//! `get_leaderboard` needs a live Postgres pool and a Discord HTTP client, so
//! it cannot be exercised from an integration test; these tests scan the
//! source text instead, matching the house style in `skeleton_fallbacks.rs`.

use std::fs;
use std::path::Path;

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// `src/server/levels.rs`, or empty text if it cannot be read; the vacuity
/// canary below catches that case.
fn server_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/server/levels.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/ui/pages/levels.rs`, or empty text if it cannot be read; the vacuity
/// canary below catches that case.
fn page_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/ui/pages/levels.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// Collapses every run of ASCII whitespace to a single space so rustfmt
/// line-wrapping cannot break a multi-token pattern match.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The UI must take `has_next` from the server response, not re-derive it
/// from the row count - that re-derivation is exactly the bug UI-11 fixed.
#[test]
fn the_ui_does_not_infer_the_next_page() {
    let page = page_source();
    let squeezed_page = squeezed(&page);

    assert!(
        !page.contains("entries.len() =="),
        "the UI must take has_next from the server rather than guess it \
         from the row count - found `entries.len() ==` in src/ui/pages/levels.rs"
    );
    assert!(
        !page.contains("PAGE_SIZE"),
        "the UI must take has_next from the server rather than guess it \
         from the row count - found a PAGE_SIZE constant in \
         src/ui/pages/levels.rs, which is exactly the duplicated copy that \
         let the UI and server drift apart"
    );
    assert!(
        squeezed_page.contains("Ok(LeaderboardPage { entries, has_next })"),
        "the UI must take has_next from the server rather than guess it \
         from the row count - expected to destructure \
         `Ok(LeaderboardPage {{ entries, has_next }})` in \
         src/ui/pages/levels.rs but did not find it"
    );
}

/// An exact `has_next` needs one row beyond the page; a hard-coded `LIMIT
/// 10` cannot tell a full page from a last page.
#[test]
fn the_server_fetches_one_row_past_the_page() {
    let squeezed_server = squeezed(&server_source());

    assert!(
        squeezed_server.contains("let limit = PAGE_SIZE + 1;"),
        "an exact has_next needs one row beyond the page - expected \
         `let limit = PAGE_SIZE + 1;` in src/server/levels.rs but did not \
         find it"
    );
    assert!(
        squeezed_server.contains("LIMIT $1 OFFSET $2"),
        "an exact has_next needs one row beyond the page - expected the \
         global-board query in src/server/levels.rs to bind `limit` via \
         `LIMIT $1 OFFSET $2` but did not find it"
    );
    assert!(
        squeezed_server.contains("LIMIT $2 OFFSET $3"),
        "an exact has_next needs one row beyond the page - expected the \
         guild-board query in src/server/levels.rs to bind `limit` via \
         `LIMIT $2 OFFSET $3` but did not find it"
    );
    assert!(
        !squeezed_server.contains("LIMIT 10"),
        "a hard-coded LIMIT 10 cannot tell a full page from a last page - \
         found `LIMIT 10` in src/server/levels.rs"
    );
}

/// The extra row exists only to answer "is there a next page"; the render
/// loop is bounded to `PAGE_SIZE` so it is never shown and never costs a
/// Discord user lookup.
#[test]
fn the_probe_row_never_reaches_the_ui() {
    let squeezed_server = squeezed(&server_source());

    assert!(
        squeezed_server.contains("(offset + 1..=offset + PAGE_SIZE).zip(&rows)"),
        "the render loop must stay bounded to PAGE_SIZE so the probe row is \
         never shown and never costs a Discord user lookup - expected \
         `(offset + 1..=offset + PAGE_SIZE).zip(&rows)` in \
         src/server/levels.rs but did not find it"
    );
    assert!(
        squeezed_server.contains("let has_next = rows.len() > entries.len();"),
        "the probe row exists only to answer whether there is a next page - \
         expected `let has_next = rows.len() > entries.len();` in \
         src/server/levels.rs but did not find it"
    );
}

/// An empty page past page 1 must still render the pager, or the user has
/// no way back.
#[test]
fn the_pager_escapes_an_empty_page() {
    let page = page_source();
    let squeezed_page = squeezed(&page);

    assert!(
        squeezed_page.contains("(page.get() > 1 || has_next).then("),
        "an empty page past page 1 must still render the pager, or the \
         user has no way back - expected \
         `(page.get() > 1 || has_next).then(` in src/ui/pages/levels.rs but \
         did not find it"
    );
    assert!(
        squeezed_page.contains("class=\"pager\""),
        "an empty page past page 1 must still render the pager, or the \
         user has no way back - expected `class=\"pager\"` in \
         src/ui/pages/levels.rs but did not find it"
    );
    assert!(
        !squeezed_page.contains("entries.is_empty() =>"),
        "the old guarded match arm that rendered the empty message with no \
         pager must not reappear - found `entries.is_empty() =>` in \
         src/ui/pages/levels.rs"
    );
}

/// Every assertion above passes trivially against a scan that has quietly
/// stopped finding anything.
#[test]
fn the_scan_still_reaches_both_files() {
    let server = server_source();
    let page = page_source();

    assert!(
        !server.is_empty(),
        "src/server/levels.rs came back empty - the scan is not reaching \
         the server module"
    );
    assert!(
        !page.is_empty(),
        "src/ui/pages/levels.rs came back empty - the scan is not reaching \
         the UI module"
    );
    assert!(
        server.contains("get_leaderboard"),
        "src/server/levels.rs did not contain get_leaderboard - the file \
         read back looks unrecognisable"
    );
    assert!(
        page.contains("LevelsPage"),
        "src/ui/pages/levels.rs did not contain LevelsPage - the file read \
         back looks unrecognisable"
    );
}
