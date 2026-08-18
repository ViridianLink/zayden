//! Regression tests for the `/shop list` pager.
//!
//! The `<` / `>` buttons carry no state: the handler re-derives the current
//! page by parsing the embed title back into a [`ShopPage`] and stepping it by
//! `+1` / `-1`. That step used to be `usize::try_from(page_change)`, which
//! turns `-1` into `0` — so `<` silently re-rendered the page you were already
//! on — while `>` past the last page fell back to the first.

use std::str::FromStr;

use gambling::ShopPage;

const FIRST: ShopPage = ShopPage::Item;
const LAST: ShopPage = ShopPage::Mine2;

/// Consecutive `(page, next_page)` pairs, without indexing.
fn adjacent_pages() -> impl Iterator<Item = (ShopPage, ShopPage)> {
    let pages = ShopPage::pages();
    pages.into_iter().zip(pages.into_iter().skip(1))
}

#[test]
fn next_advances_one_page() {
    for (from, to) in adjacent_pages() {
        assert_eq!(from.step(1), to, "`>` on {from:?} should land on {to:?}");
    }
}

/// The bug: `-1` fell through `usize::try_from` to `0`, so `<` was a no-op on
/// every page, not just the first.
#[test]
fn prev_goes_back_one_page() {
    for (to, from) in adjacent_pages() {
        assert_eq!(from.step(-1), to, "`<` on {from:?} should land on {to:?}");
    }
}

#[test]
fn stepping_clamps_at_both_ends() {
    assert_eq!(FIRST.step(-1), FIRST);
    assert_eq!(LAST.step(1), LAST);
}

#[test]
fn no_change_holds_the_page() {
    for page in ShopPage::pages() {
        assert_eq!(page.step(0), page);
    }
}

/// The pager's only channel for the current page is the embed title, which
/// `shop_response` writes as `"{page} Shop"` and reads back with
/// `strip_suffix(" Shop")` + `parse`. Every page must survive that round trip,
/// or the pager silently resets to the first page.
#[test]
fn every_page_round_trips_through_its_embed_title() {
    for page in ShopPage::pages() {
        let title = format!("{page} Shop");

        let parsed = title
            .strip_suffix(" Shop")
            .and_then(|name| ShopPage::from_str(name).ok());

        assert_eq!(parsed, Some(page), "`{title}` did not parse back to a page");
    }
}
