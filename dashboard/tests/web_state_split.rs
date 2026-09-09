//! RX-07: `WebState` used to be one flat twelve-field struct, and every
//! axum handler took `State<WebState>`, so each request cloned all twelve
//! fields (about ten allocations) and it was not statically apparent which
//! handler touched Discord, Patreon, or the database. It is now seven
//! grouped fields in `src/state.rs`, each reachable through its own
//! `FromRef<WebState>` impl, so a handler declares only the sub-states it
//! actually consumes.
//!
//! `src/main.rs`, `src/state.rs`, `src/web/` and `src/middleware/` are the
//! binary crate, unreachable from `dashboard/tests/` (which links the
//! library), so these assertions scan the source text instead, matching
//! the house style in `sidebar_module_list.rs`.

use std::fs;
use std::path::Path;

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// `src/state.rs`, or empty text if it cannot be read; the vacuity canary
/// below catches that case.
fn state_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/state.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/main.rs`, or empty text if it cannot be read; the vacuity canary
/// below catches that case.
fn main_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/main.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/web/mod.rs`, or empty text if it cannot be read; the vacuity
/// canary below catches that case.
fn web_mod_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/web/mod.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/middleware/auth.rs`, or empty text if it cannot be read; the
/// vacuity canary below catches that case.
fn auth_middleware_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/middleware/auth.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/web/routes_login.rs`, or empty text if it cannot be read; the
/// vacuity canary below catches that case.
fn login_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/web/routes_login.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/web/routes_kofi.rs`, or empty text if it cannot be read; the
/// vacuity canary below catches that case.
fn kofi_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/web/routes_kofi.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/web/routes_patreon.rs`, or empty text if it cannot be read; the
/// vacuity canary below catches that case.
fn patreon_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/web/routes_patreon.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// Collapses every run of ASCII whitespace to a single space so rustfmt
/// line-wrapping cannot break a multi-token pattern match.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

const SUB_STATES: [&str; 7] = [
    "Arc<ZaydenAppState>",
    "SessionState",
    "Arc<OAuthState>",
    "DiscordState",
    "Arc<IntegrationsState>",
    "Arc<SiteUrls>",
    "LeptosOptions",
];

/// A handler that takes `State<WebState>` clones all twelve original
/// fields per request and hides which subsystems it actually touches -
/// the exact cost and opacity RX-07 removed.
#[test]
fn no_handler_receives_the_whole_state() {
    let files: [(&str, String); 6] = [
        ("src/main.rs", main_source()),
        ("src/web/mod.rs", web_mod_source()),
        ("src/middleware/auth.rs", auth_middleware_source()),
        ("src/web/routes_login.rs", login_source()),
        ("src/web/routes_kofi.rs", kofi_source()),
        ("src/web/routes_patreon.rs", patreon_source()),
    ];

    for (path, source) in &files {
        let squeezed_source = squeezed(source);
        assert!(
            !squeezed_source.contains("State<WebState>"),
            "{path} takes State<WebState> - a handler on the whole state \
             clones all twelve original fields per request and hides \
             which subsystems it touches, which is the cost RX-07 \
             removed"
        );
    }
}

/// Every sub-state must have a `FromRef<WebState>` impl, or it is
/// unreachable as an extractor and the handler that needs it is forced
/// back onto the whole `WebState`.
#[test]
fn every_substate_is_reachable_from_the_root() {
    let squeezed_state = squeezed(&state_source());

    for sub_state in SUB_STATES {
        let pattern = format!("impl FromRef<WebState> for {sub_state}");
        assert!(
            squeezed_state.contains(&pattern),
            "src/state.rs is missing `{pattern}` - without this impl \
             {sub_state} is unreachable as an extractor and any handler \
             that needs it is forced back onto the whole WebState"
        );
    }
}

/// `from_fn_with_state` clones its state argument on every protected
/// request, so it must be handed only the session, not the whole
/// `WebState`.
#[test]
fn the_auth_middleware_carries_only_the_session() {
    let squeezed_web_mod = squeezed(&web_mod_source());
    let squeezed_auth = squeezed(&auth_middleware_source());

    assert!(
        squeezed_web_mod.contains(
            "from_fn_with_state( SessionState::from_ref(state), \
             require_auth,"
        ),
        "src/web/mod.rs does not wire require_auth through \
         from_fn_with_state(SessionState::from_ref(state), ...) - \
         from_fn_with_state clones its state on every protected request, \
         so handing it the whole WebState would put a twelve-field clone \
         on the hot path for every authenticated route"
    );
    assert!(
        squeezed_auth.contains("State(session): State<SessionState>"),
        "src/middleware/auth.rs does not extract State<SessionState> - \
         require_auth must only depend on the session, not the whole \
         WebState"
    );
}

/// A module that names a sub-state it never uses has re-acquired the
/// opacity RX-07 removed - the boundaries must stay statically visible.
#[test]
fn the_route_modules_only_name_what_they_use() {
    let kofi = kofi_source();
    let login = login_source();
    let patreon = patreon_source();

    assert!(
        !kofi.contains("OAuthState"),
        "src/web/routes_kofi.rs names OAuthState but never uses it - a \
         module that names a sub-state it never touches has re-acquired \
         the opacity RX-07 removed"
    );
    assert!(
        !kofi.contains("DiscordState"),
        "src/web/routes_kofi.rs names DiscordState but never uses it - a \
         module that names a sub-state it never touches has re-acquired \
         the opacity RX-07 removed"
    );
    assert!(
        !login.contains("IntegrationsState"),
        "src/web/routes_login.rs names IntegrationsState but never uses \
         it - a module that names a sub-state it never touches has \
         re-acquired the opacity RX-07 removed"
    );
    assert!(
        !login.contains("DiscordState"),
        "src/web/routes_login.rs names DiscordState but never uses it - \
         a module that names a sub-state it never touches has \
         re-acquired the opacity RX-07 removed"
    );
    assert!(
        !patreon.contains("OAuthState"),
        "src/web/routes_patreon.rs names OAuthState but never uses it - \
         a module that names a sub-state it never touches has \
         re-acquired the opacity RX-07 removed"
    );
}

/// These three groups hold owned Strings - the OAuth client id/secret/
/// urls, the Patreon app credentials and webhook URI, and the invite and
/// upgrade URLs. Behind an `Arc` a per-request extraction is one
/// refcount bump; without it every request re-allocates those strings,
/// which is precisely the cost RX-07 was raised about.
#[test]
fn the_shared_config_groups_stay_behind_an_arc() {
    let squeezed_state = squeezed(&state_source());

    assert!(
        squeezed_state.contains("oauth: Arc<OAuthState>"),
        "src/state.rs does not hold oauth behind an Arc<OAuthState> - \
         without it every request re-allocates the OAuth client \
         id/secret/urls instead of bumping a refcount"
    );
    assert!(
        squeezed_state.contains("integrations: Arc<IntegrationsState>"),
        "src/state.rs does not hold integrations behind an \
         Arc<IntegrationsState> - without it every request re-allocates \
         the Patreon app credentials and webhook URI instead of bumping \
         a refcount"
    );
    assert!(
        squeezed_state.contains("urls: Arc<SiteUrls>"),
        "src/state.rs does not hold urls behind an Arc<SiteUrls> - \
         without it every request re-allocates the invite and upgrade \
         URLs instead of bumping a refcount"
    );
}

/// Every assertion above passes trivially against a scan that has
/// quietly stopped finding anything.
#[test]
fn the_scan_still_reaches_every_file() {
    let state = state_source();
    let main = main_source();
    let web_mod = web_mod_source();
    let auth = auth_middleware_source();
    let login = login_source();
    let kofi = kofi_source();
    let patreon = patreon_source();

    assert!(
        !state.is_empty(),
        "src/state.rs came back empty - the scan is not reaching state.rs"
    );
    assert!(
        !main.is_empty(),
        "src/main.rs came back empty - the scan is not reaching main.rs"
    );
    assert!(
        !web_mod.is_empty(),
        "src/web/mod.rs came back empty - the scan is not reaching \
         web/mod.rs"
    );
    assert!(
        !auth.is_empty(),
        "src/middleware/auth.rs came back empty - the scan is not \
         reaching middleware/auth.rs"
    );
    assert!(
        !login.is_empty(),
        "src/web/routes_login.rs came back empty - the scan is not \
         reaching routes_login.rs"
    );
    assert!(
        !kofi.is_empty(),
        "src/web/routes_kofi.rs came back empty - the scan is not \
         reaching routes_kofi.rs"
    );
    assert!(
        !patreon.is_empty(),
        "src/web/routes_patreon.rs came back empty - the scan is not \
         reaching routes_patreon.rs"
    );

    assert!(
        state.contains("struct WebState"),
        "src/state.rs did not contain struct WebState - the file read \
         back looks unrecognisable"
    );
    assert!(
        main.contains("async fn main"),
        "src/main.rs did not contain async fn main - the file read back \
         looks unrecognisable"
    );
    assert!(
        web_mod.contains("fn routes"),
        "src/web/mod.rs did not contain fn routes - the file read back \
         looks unrecognisable"
    );
    assert!(
        auth.contains("async fn require_auth"),
        "src/middleware/auth.rs did not contain async fn require_auth - \
         the file read back looks unrecognisable"
    );
    assert!(
        login.contains("logout_handler"),
        "src/web/routes_login.rs did not contain logout_handler - the \
         file read back looks unrecognisable"
    );
    assert!(
        kofi.contains("kofi_webhook_handler"),
        "src/web/routes_kofi.rs did not contain kofi_webhook_handler - \
         the file read back looks unrecognisable"
    );
    assert!(
        patreon.contains("patreon_callback_handler"),
        "src/web/routes_patreon.rs did not contain \
         patreon_callback_handler - the file read back looks \
         unrecognisable"
    );
}
