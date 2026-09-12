//! Letterboxd RSS parsing. The feed carries `<tmdb:movieId>`, which is what
//! makes the library cross-reference an exact join rather than a title guess.

use jellyfin::discovery::letterboxd::{feed_url, parse};

const FEED: &str = r#"<?xml version='1.0' encoding='utf-8'?>
<rss version="2.0" xmlns:letterboxd="https://letterboxd.com" xmlns:tmdb="https://themoviedb.org">
  <channel>
    <title>Letterboxd - Dave</title>
    <item>
      <title>Blue Heron, 2025 - ★★★★</title>
      <letterboxd:filmTitle>Blue Heron</letterboxd:filmTitle>
      <letterboxd:filmYear>2025</letterboxd:filmYear>
      <letterboxd:memberRating>4.0</letterboxd:memberRating>
      <letterboxd:watchedDate>2026-09-08</letterboxd:watchedDate>
      <letterboxd:rewatch>No</letterboxd:rewatch>
      <tmdb:movieId>1184941</tmdb:movieId>
    </item>
    <item>
      <title>Novocaine, 2025</title>
      <letterboxd:filmTitle>Novocaine</letterboxd:filmTitle>
      <letterboxd:filmYear>2025</letterboxd:filmYear>
      <letterboxd:rewatch>Yes</letterboxd:rewatch>
    </item>
  </channel>
</rss>"#;

#[test]
fn diary_entries_are_extracted() {
    let entries = parse(FEED).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn a_full_entry_keeps_every_field() {
    let entries = parse(FEED).unwrap();
    let first = &entries[0];

    assert_eq!(first.title, "Blue Heron");
    assert_eq!(first.year, Some(2025));
    assert_eq!(first.tmdb_id, Some(1_184_941));
    assert_eq!(first.rating, Some(4.0));
    assert_eq!(first.watched_date.as_deref(), Some("2026-09-08"));
    assert!(!first.rewatch);
}

#[test]
fn a_missing_tmdb_id_is_none_rather_than_an_error() {
    // These are the entries that fall back to title matching, and the gap
    // report has to disclose them separately.
    let entries = parse(FEED).unwrap();
    let second = &entries[1];

    assert_eq!(second.tmdb_id, None);
    assert_eq!(second.rating, None);
    assert!(second.rewatch);
}

#[test]
fn an_empty_feed_is_not_an_error() {
    let empty = r"<?xml version='1.0'?><rss><channel></channel></rss>";
    assert_eq!(parse(empty).unwrap(), Vec::new());
}

#[test]
fn feed_urls_tolerate_stray_slashes() {
    assert_eq!(feed_url("dave"), "https://letterboxd.com/dave/rss/");
    assert_eq!(feed_url(" /dave/ "), "https://letterboxd.com/dave/rss/");
}
