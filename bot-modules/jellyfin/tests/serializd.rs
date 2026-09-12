//! Serializd diary parsing. Every entry carries the show's TMDB id, so the
//! library cross-reference is an exact join; ratings arrive out of ten.

use jellyfin::discovery::serializd::{diary_url, parse, stars};

const PAGE: &str = r#"{
  "totalPages": 2,
  "totalReviews": 3,
  "reviews": [
    {
      "id": 11,
      "showId": 1396,
      "seasonId": 3572,
      "seasonName": "Season 1",
      "rating": 9,
      "showName": "Breaking Bad",
      "showPremiereDate": "2008-01-20",
      "backdate": "2026-09-01",
      "dateAdded": "2026-09-02T10:00:00Z",
      "isRewatched": false,
      "isLogged": true,
      "episodeNumber": 4,
      "showSeasons": [{ "id": 3572, "name": "Season 1", "seasonNumber": 1 }]
    },
    {
      "id": 12,
      "showId": 136315,
      "rating": 0,
      "showName": "The Bear",
      "showPremiereDate": null,
      "backdate": null,
      "dateAdded": "2026-08-30T10:00:00Z",
      "isRewatched": true
    },
    { "id": 13, "showId": 0, "rating": 8, "showName": "No show id" },
    { "id": 14, "showId": 1399, "rating": 8, "showName": "  " }
  ]
}"#;

#[test]
fn entries_with_a_show_are_extracted() {
    let page = parse(PAGE).unwrap();
    assert_eq!(page.entries.len(), 2);
    assert_eq!(page.total_pages, 2);
}

#[test]
fn a_full_entry_keeps_every_field() {
    let page = parse(PAGE).unwrap();
    let first = &page.entries[0];

    assert_eq!(first.tmdb_id, 1396);
    assert_eq!(first.title, "Breaking Bad");
    assert_eq!(first.year, Some(2008));
    assert_eq!(first.rating, Some(4.5));
    assert_eq!(first.logged_date.as_deref(), Some("2026-09-01"));
    assert!(!first.rewatch);
}

#[test]
fn an_unrated_entry_has_no_rating_and_falls_back_to_the_added_date() {
    let page = parse(PAGE).unwrap();
    let second = &page.entries[1];

    assert_eq!(second.rating, None);
    assert_eq!(second.year, None);
    assert_eq!(second.logged_date.as_deref(), Some("2026-08-30T10:00:00Z"));
    assert!(second.rewatch);
}

#[test]
fn a_candidate_carries_the_show_id() {
    let page = parse(PAGE).unwrap();
    assert_eq!(page.entries[0].candidate().tmdb_id, Some(1396));
}

#[test]
fn ratings_out_of_ten_become_stars() {
    assert_eq!(stars(10), Some(5.0));
    assert_eq!(stars(7), Some(3.5));
    assert_eq!(stars(1), Some(0.5));
    assert_eq!(stars(0), None);
    assert_eq!(stars(11), None);
    assert_eq!(stars(-4), None);
}

#[test]
fn an_empty_diary_is_not_an_error() {
    let page = parse(r#"{ "reviews": [], "totalPages": 0 }"#).unwrap();
    assert_eq!(page.entries, Vec::new());
    assert_eq!(page.total_pages, 0);
}

#[test]
fn a_missing_page_count_means_one_page() {
    assert_eq!(parse(r#"{ "reviews": [] }"#).unwrap().total_pages, 1);
}

#[test]
fn malformed_json_is_an_error() {
    assert!(parse("<html>502 Bad Gateway</html>").is_err());
}

#[test]
fn diary_urls_tolerate_stray_slashes() {
    assert_eq!(
        diary_url(" /dave/ ", 1),
        "https://serializd.onrender.com/api/user/dave/diary?page=1"
    );
}

#[test]
fn diary_urls_cannot_escape_the_username_segment() {
    assert_eq!(
        diary_url("a b/../c?d", 3),
        "https://serializd.onrender.com/api/user/a%20b%2F..%2Fc%3Fd/diary?page=3"
    );
}
