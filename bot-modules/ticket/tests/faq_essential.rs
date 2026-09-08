//! Which wiki pages ride along on every triage embed.
//!
//! A support wiki's FAQ and troubleshooting pages answer more tickets than any
//! keyword match will, so they are offered whether or not the search found
//! them. The rules worth pinning are what counts as one of those pages and,
//! when a wiki has several, which one wins.

use ticket::faq::essential::select;
use ticket::wiki::PageListItem;

fn page(id: i32, path: &str, title: Option<&str>) -> PageListItem {
    PageListItem {
        id,
        path: String::from(path),
        title: title.map(String::from),
        description: None,
        is_published: true,
    }
}

#[test]
fn both_pages_are_offered_in_a_fixed_order() {
    let listed = [
        page(1, "troubleshooting", Some("Troubleshooting")),
        page(2, "faq", Some("FAQ")),
        page(3, "install", Some("Installing")),
    ];

    let picked = select(&listed);

    assert_eq!(picked.len(), 2);
    assert_eq!(picked[0].path, "faq");
    assert_eq!(picked[1].path, "troubleshooting");
}

/// A wiki that only has one of them offers only that one, rather than inventing
/// a link to a page that is not there.
#[test]
fn a_missing_page_is_simply_absent() {
    let listed = [page(1, "faq", Some("FAQ"))];

    let picked = select(&listed);

    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].path, "faq");
}

#[test]
fn a_wiki_with_neither_page_offers_nothing() {
    let listed = [page(1, "install", Some("Installing"))];

    assert_eq!(select(&listed), []);
}

/// The whole-wiki FAQ beats a per-game one, so a Palworld ticket is not sent to
/// the Minecraft FAQ just because it was listed first.
#[test]
fn the_shallowest_page_wins() {
    let listed = [
        page(1, "games/minecraft/faq", Some("Minecraft FAQ")),
        page(2, "faq", Some("FAQ")),
        page(3, "games/palworld/faq", Some("Palworld FAQ")),
    ];

    let picked = select(&listed);

    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].path, "faq");
}

#[test]
fn a_page_is_recognised_by_its_title_as_well_as_its_path() {
    let listed = [page(1, "help/common-questions", Some("FAQ"))];

    let picked = select(&listed);

    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].path, "help/common-questions");
}

/// Wiki.js paths are slugs, so the match has to survive punctuation and case
/// the author chose freely.
#[test]
fn punctuation_and_case_do_not_hide_a_page() {
    let listed = [page(1, "docs/Trouble-Shooting", None)];

    let picked = select(&listed);

    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].path, "docs/Trouble-Shooting");
}

#[test]
fn a_near_miss_is_not_treated_as_the_page() {
    let listed = [
        page(1, "faq-archive", Some("Old FAQ")),
        page(2, "troubleshooting-networking", Some("Network troubleshooting")),
    ];

    assert_eq!(select(&listed), []);
}

/// The wiki's own title is what the reader sees; the slug is only the fallback.
#[test]
fn the_pages_own_title_is_used_when_it_has_one() {
    let listed = [
        page(1, "faq", Some("Frequently Asked Questions")),
        page(2, "troubleshooting", None),
    ];

    let picked = select(&listed);

    assert_eq!(picked[0].title, "Frequently Asked Questions");
    assert_eq!(picked[1].title, "Troubleshooting");
}
