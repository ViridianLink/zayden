use web::shell::link::{
    aria_current,
    is_active_for,
    module_list_location,
    sidebar_active,
};

#[test]
fn a_link_is_current_on_its_own_page_and_below_it() {
    assert!(is_active_for("/", "/"));
    assert!(is_active_for("/guilds", "/guilds"));
    assert!(is_active_for("/guild/7", "/guild/7"));
    assert!(is_active_for("/guild/7", "/guild/7/settings/general"));
    assert!(is_active_for("/guild/7", "/guild/7/"));
    assert!(is_active_for(
        "/admin/destiny2/loadouts",
        "/admin/destiny2/loadouts/new"
    ));
}

#[test]
fn a_link_is_not_current_above_or_beside_it() {
    assert!(!is_active_for("/guild/7/settings/general", "/guild/7/settings"));
    assert!(!is_active_for("/guild/7/settings/general", "/guild/7"));
    assert!(!is_active_for("/guild/7", "/guild/70"));
    assert!(!is_active_for("/guild/7", "/guild/8/levels"));
    assert!(!is_active_for("/guilds", "/guild/7"));
    assert!(!is_active_for("/upgrade", "/guilds"));
}

#[test]
fn a_trailing_slash_on_the_link_is_optional() {
    assert!(is_active_for("/item/", "/item"));
    assert!(is_active_for("/item/", "/item/"));
    assert!(is_active_for("/item/", "/item/one"));
}

#[test]
fn the_root_link_is_current_only_at_the_root() {
    assert!(is_active_for("/", "/"));
    assert!(!is_active_for("/", "/guilds"));
    assert!(!is_active_for("/", "/guild/7/levels"));
}

#[test]
fn aria_current_names_the_page() {
    assert_eq!(aria_current("/guild/7", "/guild/7/levels"), Some("page"));
    assert_eq!(aria_current("/guild/7/levels", "/guild/7"), None);
}

#[test]
fn exact_sidebar_entries_match_only_their_page() {
    assert!(sidebar_active("/guilds", "/guilds", true));
    assert!(!sidebar_active("/admin/servers", "/admin/servers/x", true));
    assert!(sidebar_active(
        "/admin/destiny2/loadouts",
        "/admin/destiny2/loadouts/new",
        false
    ));
    assert!(sidebar_active("/upgrade", "/upgrade", false));
    assert!(!sidebar_active("/upgrade", "/guilds", false));
}

#[test]
fn the_bare_settings_page_highlights_general() {
    assert_eq!(
        module_list_location("7", "/guild/7/settings"),
        "/guild/7/settings/general"
    );
    assert_eq!(
        module_list_location("7", "/guild/7/settings/ai"),
        "/guild/7/settings/ai"
    );
    assert_eq!(
        module_list_location("7", "/guild/7/settings/bogus"),
        "/guild/7/settings/bogus"
    );
    assert_eq!(module_list_location("7", "/guild/7"), "/guild/7");
}
