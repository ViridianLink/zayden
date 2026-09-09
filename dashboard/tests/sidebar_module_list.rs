//! RX-05 "Caret toggle destroys and rebuilds the module list; orphan context
//! fallback" in `src/ui/components/layout.rs`. Two defects, now fixed. First,
//! the `ModulesGroup` sublist closure read `open.get()` at its top, so
//! collapsing or expanding the sidebar group tore down and reconstructed all
//! 12 `<A>` module links, each with its own reactive class closure, instead
//! of just toggling visibility; visibility is now driven by a CSS class on a
//! permanently-mounted `<div>`. Second, `use_context::<ModulesOpen>()` fell
//! back to `RwSignal::new(true)` when the context was missing, silently
//! creating an orphan signal no other component observes, so a
//! context-wiring regression would have looked like a toggle that works but
//! does not persist; the consumer now uses `expect_context`, and the app
//! root provides the context unconditionally.
//! `ModulesGroup` needs router and context wiring to instantiate, so it
//! cannot be exercised from an integration test; these tests scan the
//! source text instead, matching the house style in
//! `leaderboard_pagination.rs`.

use std::fs;
use std::path::Path;

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// `src/ui/components/layout.rs`, or empty text if it cannot be read; the
/// vacuity canary below catches that case.
fn layout_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/ui/components/layout.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/app.rs`, or empty text if it cannot be read; the vacuity canary
/// below catches that case.
fn app_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/app.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `style/partials/layout.css`, or empty text if it cannot be read; the
/// vacuity canary below catches that case.
fn layout_css() -> String {
    let path = Path::new(CRATE_ROOT).join("style/partials/layout.css");
    fs::read_to_string(path).unwrap_or_default()
}

/// Collapses every run of ASCII whitespace to a single space so rustfmt
/// line-wrapping cannot break a multi-token pattern match.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Toggling the caret must change a class, not rebuild the subtree.
#[test]
fn the_caret_toggle_keeps_the_links_mounted() {
    let source = layout_source();
    let squeezed_source = squeezed(&source);

    assert!(
        !source.contains("open.get().then("),
        "toggling the caret must change a class, not rebuild the subtree - \
         found the conditional-render pattern `open.get().then(` in \
         src/ui/components/layout.rs"
    );
    assert!(
        squeezed_source.contains("\"app-sidebar-sublist open\""),
        "toggling the caret must change a class, not rebuild the subtree - \
         expected the string literal `\"app-sidebar-sublist open\"` in \
         src/ui/components/layout.rs but did not find it"
    );
    assert!(
        squeezed_source.contains("<div class=sublist_class>"),
        "toggling the caret must change a class, not rebuild the subtree - \
         expected a permanently-mounted `<div class=sublist_class>` in \
         src/ui/components/layout.rs but did not find it"
    );
}

/// The Rust side only swaps a class - if the CSS does not actually hide
/// the collapsed list, the caret silently does nothing.
#[test]
fn the_collapsed_sublist_is_hidden_by_css() {
    let squeezed_css = squeezed(&layout_css());

    assert!(
        squeezed_css.contains(".app-sidebar-sublist { display: none;"),
        "the Rust side only swaps a class - if the collapsed rule does not \
         set `display: none;` in style/partials/layout.css the caret \
         silently does nothing"
    );
    assert!(
        squeezed_css.contains(".app-sidebar-sublist.open { display: flex;"),
        "the Rust side only swaps a class - if the open rule does not set \
         `display: flex;` in style/partials/layout.css the caret silently \
         does nothing"
    );
}

/// A missing context must fail immediately rather than mint an orphan
/// signal that no other component observes.
#[test]
fn a_missing_toggle_context_is_loud() {
    let source = layout_source();
    let squeezed_source = squeezed(&source);

    assert!(
        squeezed_source.contains("expect_context::<ModulesOpen>()"),
        "a missing context must fail immediately - expected \
         `expect_context::<ModulesOpen>()` in src/ui/components/layout.rs \
         but did not find it"
    );
    assert!(
        !source.contains("RwSignal::new(true)"),
        "a missing context must fail immediately rather than mint an \
         orphan signal that no other component observes - found the \
         orphan fallback `RwSignal::new(true)` in \
         src/ui/components/layout.rs"
    );
}

/// Dropping the fallback is only safe because the root provides the
/// context unconditionally - this test is what makes that dependency
/// explicit.
#[test]
fn the_app_root_provides_the_toggle_state() {
    let squeezed_app = squeezed(&app_source());

    assert!(
        squeezed_app.contains("provide_context(ModulesOpen(RwSignal::new(true)));"),
        "dropping the orphan-signal fallback is only safe because the root \
         provides the context unconditionally - expected \
         `provide_context(ModulesOpen(RwSignal::new(true)));` in \
         src/app.rs but did not find it"
    );
}

/// Every assertion above passes trivially against a scan that has quietly
/// stopped finding anything.
#[test]
fn the_scan_still_reaches_every_file() {
    let layout = layout_source();
    let app = app_source();
    let css = layout_css();

    assert!(
        !layout.is_empty(),
        "src/ui/components/layout.rs came back empty - the scan is not \
         reaching the layout module"
    );
    assert!(
        !app.is_empty(),
        "src/app.rs came back empty - the scan is not reaching the app root"
    );
    assert!(
        !css.is_empty(),
        "style/partials/layout.css came back empty - the scan is not \
         reaching the layout stylesheet"
    );
    assert!(
        layout.contains("ModulesGroup"),
        "src/ui/components/layout.rs did not contain ModulesGroup - the \
         file read back looks unrecognisable"
    );
    assert!(
        app.contains("provide_context"),
        "src/app.rs did not contain provide_context - the file read back \
         looks unrecognisable"
    );
    assert!(
        css.contains("app-sidebar-caret"),
        "style/partials/layout.css did not contain app-sidebar-caret - the \
         file read back looks unrecognisable"
    );
}
