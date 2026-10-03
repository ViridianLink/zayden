//! The leaderboard's additive `?scope=global&page=N` view state.

use web::engagement::{GLOBAL_SCOPE, LeaderboardView};

fn view(scope: Option<&str>, page: Option<&str>) -> LeaderboardView {
    LeaderboardView::parse(scope, page)
}

#[test]
fn the_bare_path_is_this_servers_first_page() {
    let default = view(None, None);

    assert_eq!(default, LeaderboardView { global: false, page: 1 });
    assert_eq!(default, LeaderboardView::default());
    assert_eq!(default.href("7"), "/guild/7/levels");
}

#[test]
fn only_the_exact_global_scope_selects_the_global_board() {
    assert!(view(Some("global"), None).global);
    assert!(!view(Some("Global"), None).global);
    assert!(!view(Some(" global"), None).global);
    assert!(!view(Some("guild"), None).global);
    assert!(!view(Some(""), None).global);
    assert_eq!(GLOBAL_SCOPE, "global");
}

#[test]
fn pages_are_whole_numbers_of_at_least_one() {
    assert_eq!(view(None, Some("3")).page, 3);
    assert_eq!(view(None, Some(" 4 ")).page, 4);
    assert_eq!(view(None, Some("1")).page, 1);
    assert_eq!(view(None, Some("0")).page, 1);
    assert_eq!(view(None, Some("-5")).page, 1);
    assert_eq!(view(None, Some("2.5")).page, 1);
    assert_eq!(view(None, Some("abc")).page, 1);
    assert_eq!(view(None, Some("")).page, 1);
    assert_eq!(view(None, Some("99999999999")).page, 1);
    assert_eq!(view(None, Some("2147483647")).page, i32::MAX);
}

#[test]
fn a_query_string_reads_the_same_as_its_parts() {
    let read = LeaderboardView::from_query_string;

    assert_eq!(read(None), LeaderboardView::default());
    assert_eq!(read(Some("")), LeaderboardView::default());
    assert_eq!(read(Some("scope=global&page=2")), LeaderboardView {
        global: true,
        page: 2
    });
    assert_eq!(read(Some("page=2&scope=global")), LeaderboardView {
        global: true,
        page: 2
    });
    assert_eq!(read(Some("other=1&page=4")), LeaderboardView {
        global: false,
        page: 4
    });
    assert_eq!(read(Some("page=%33")), LeaderboardView { global: false, page: 3 });
    assert_eq!(read(Some("page=")), LeaderboardView::default());
    assert_eq!(read(Some("scope&page")), LeaderboardView::default());
}

#[test]
fn a_repeated_key_reads_as_its_first_value() {
    assert_eq!(
        LeaderboardView::from_query_string(Some(
            "page=2&page=3&scope=global&scope=x"
        )),
        LeaderboardView { global: true, page: 2 }
    );
    assert_eq!(
        LeaderboardView::from_query_string(Some("scope=x&scope=global")),
        LeaderboardView::default()
    );
}

#[test]
fn each_view_has_one_address() {
    assert_eq!(view(None, None).href("7"), "/guild/7/levels");
    assert_eq!(view(Some("global"), None).href("7"), "/guild/7/levels?scope=global");
    assert_eq!(view(None, Some("3")).href("7"), "/guild/7/levels?page=3");
    assert_eq!(
        view(Some("global"), Some("3")).href("7"),
        "/guild/7/levels?scope=global&page=3"
    );
}

#[test]
fn an_address_reads_back_as_the_same_view() {
    for (global, page) in [(false, 1), (true, 1), (false, 9), (true, 12)] {
        let original = LeaderboardView { global, page };
        let href = original.href("7");
        let query = href.split_once('?').map_or("", |(_, query)| query);
        let value = |name: &str| {
            query.split('&').find_map(|pair| pair.strip_prefix(&format!("{name}=")))
        };

        assert_eq!(view(value("scope"), value("page")), original);
    }
}

#[test]
fn the_scope_buttons_start_from_page_one() {
    assert_eq!(LeaderboardView::with_scope(false).href("7"), "/guild/7/levels");
    assert_eq!(
        LeaderboardView::with_scope(true).href("7"),
        "/guild/7/levels?scope=global"
    );
}

#[test]
fn previous_is_disabled_on_the_first_page() {
    assert_eq!(view(None, None).previous(), None);
    assert_eq!(
        view(Some("global"), Some("3")).previous(),
        Some(LeaderboardView { global: true, page: 2 })
    );
    assert_eq!(
        view(None, Some("2")).previous().map(|v| v.href("7")),
        Some("/guild/7/levels".to_owned())
    );
}

#[test]
fn next_follows_what_the_response_said() {
    assert_eq!(view(None, None).next(false), None);
    assert_eq!(
        view(None, None).next(true),
        Some(LeaderboardView { global: false, page: 2 })
    );
    assert_eq!(
        view(None, Some("2147483647")).next(true).map(|v| v.page),
        Some(i32::MAX)
    );
}

#[test]
fn the_pager_shows_past_page_one_or_with_a_next_page() {
    assert!(!view(None, None).shows_pager(false));
    assert!(view(None, None).shows_pager(true));
    assert!(view(None, Some("2")).shows_pager(false));
    assert!(view(None, Some("2")).shows_pager(true));
}

#[test]
fn an_empty_board_says_why() {
    assert_eq!(
        view(None, None).empty_text(),
        "No one has chatted here yet - the board fills as members talk."
    );
    assert_eq!(
        view(Some("global"), None).empty_text(),
        "No one has earned global XP yet."
    );
    assert_eq!(view(None, Some("2")).empty_text(), "No more entries on this page.");
    assert_eq!(
        view(Some("global"), Some("2")).empty_text(),
        "No more entries on this page."
    );
}
