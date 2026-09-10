//! The reaper deletes Jellyfin users and libraries on the strength of these
//! predicates alone, so a loose match would delete a real member's account.

use jellyfin::guest::naming::{
    guest_username,
    is_managed_guest,
    is_managed_library,
    library_name,
    party_id_of_guest,
    party_id_of_library,
};

#[test]
fn generated_names_are_recognised() {
    assert!(is_managed_guest(&guest_username(42, 211_486_447_369_322_506)));
    assert!(is_managed_library(&library_name(42)));
}

#[test]
fn a_real_user_who_looks_like_a_guest_is_not_matched() {
    // The whole safety property in one case.
    assert!(!is_managed_guest("party-guy"));
    assert!(!is_managed_guest("zayden-party-guy"));
    assert!(!is_managed_guest("zayden-party-guy-123"));
    assert!(!is_managed_guest("zayden-party-123-abc"));
    assert!(!is_managed_guest("zayden-party-"));
    assert!(!is_managed_guest("zayden-party-123"));
}

#[test]
fn unrelated_users_are_never_matched() {
    for name in ["admin", "Sierra", "Zayden", "zayden", "guest", "Guest"] {
        assert!(!is_managed_guest(name), "`{name}` must not be matched");
    }
}

#[test]
fn a_real_library_is_not_matched() {
    for name in [
        "Movies",
        "Shows",
        "Ruby's Recommended Movies",
        "Zayden Party",
        "Zayden Party abc",
    ] {
        assert!(!is_managed_library(name), "`{name}` must not be matched");
    }
}

#[test]
fn party_ids_round_trip() {
    assert_eq!(party_id_of_guest(&guest_username(7, 99)), Some(7));
    assert_eq!(party_id_of_library(&library_name(7)), Some(7));
    assert_eq!(party_id_of_guest("party-guy"), None);
    assert_eq!(party_id_of_library("Movies"), None);
}
