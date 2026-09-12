//! Diaries log the same title many times (rewatches, and every episode of a
//! show), so the gap report collapses them before counting or listing.

use jellyfin::discovery::gaps::{Candidate, collapse};

fn candidate(title: &str, tmdb_id: Option<i32>, rating: Option<f32>) -> Candidate {
    Candidate { title: title.to_owned(), year: None, tmdb_id, rating }
}

#[test]
fn repeated_tmdb_ids_collapse_to_one() {
    let collapsed = collapse(vec![
        candidate("Breaking Bad", Some(1396), None),
        candidate("Breaking Bad", Some(1396), None),
        candidate("The Bear", Some(136_315), None),
    ]);

    assert_eq!(collapsed.len(), 2);
}

#[test]
fn the_best_rating_survives() {
    let collapsed = collapse(vec![
        candidate("Breaking Bad", Some(1396), Some(3.0)),
        candidate("Breaking Bad", Some(1396), None),
        candidate("Breaking Bad", Some(1396), Some(4.5)),
    ]);

    assert_eq!(collapsed[0].rating, Some(4.5));
}

#[test]
fn a_rating_on_a_later_entry_is_kept() {
    let collapsed = collapse(vec![
        candidate("Novocaine", Some(1), None),
        candidate("Novocaine", Some(1), Some(2.5)),
    ]);

    assert_eq!(collapsed[0].rating, Some(2.5));
}

#[test]
fn title_only_entries_collapse_ignoring_case() {
    let collapsed = collapse(vec![
        candidate("Blue Heron", None, None),
        candidate("blue heron", None, Some(4.0)),
    ]);

    assert_eq!(collapsed.len(), 1);
    assert_eq!(collapsed[0].title, "Blue Heron");
    assert_eq!(collapsed[0].rating, Some(4.0));
}

#[test]
fn a_shared_title_with_different_ids_stays_apart() {
    let collapsed = collapse(vec![
        candidate("The Office", Some(2316), None),
        candidate("The Office", Some(2996), None),
        candidate("The Office", None, None),
    ]);

    assert_eq!(collapsed.len(), 3);
}

#[test]
fn first_seen_order_is_kept() {
    let collapsed = collapse(vec![
        candidate("B", Some(2), None),
        candidate("A", Some(1), None),
        candidate("B", Some(2), None),
    ]);

    let titles: Vec<&str> = collapsed.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["B", "A"]);
}
