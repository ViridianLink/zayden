//! Integration tests for the M6b news/social de-dup logic in `news.rs`.

use marathon::model::NewsItem;
use marathon::news;

fn item(id: &str) -> NewsItem {
    NewsItem {
        feed_key: "bungie_news".to_string(),
        id: id.to_string(),
        source_label: "Bungie News".to_string(),
        title: format!("Item {id}"),
        url: None,
        summary: None,
    }
}

#[test]
fn new_since_returns_nothing_on_cold_start() {
    let items = vec![item("3"), item("2"), item("1")];

    assert!(news::new_since(&items, None).is_empty());
}

#[test]
fn new_since_returns_items_newer_than_last_id() {
    let items = vec![item("3"), item("2"), item("1")];

    let new_items = news::new_since(&items, Some("2"));

    assert_eq!(new_items, [item("3")]);
}

#[test]
fn new_since_returns_nothing_when_last_id_is_newest() {
    let items = vec![item("3"), item("2"), item("1")];

    assert!(news::new_since(&items, Some("3")).is_empty());
}

#[test]
fn new_since_treats_unknown_last_id_as_everything_new() {
    let items = vec![item("3"), item("2"), item("1")];

    let new_items = news::new_since(&items, Some("stale-rotated-out-id"));

    assert_eq!(new_items, items);
}
