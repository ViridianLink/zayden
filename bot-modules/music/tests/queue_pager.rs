//! Discord rejects a message whose action row contains two components with the
//! same `custom_id` (`COMPONENT_CUSTOM_ID_DUPLICATED`, JSON error 50035), even
//! when both are disabled. The pager encodes its target page into the id, so a
//! single-page queue used to collapse "Previous" and "Next" onto page 0 and the
//! whole `/music queue` response failed to send.

use music::components::QueuePager;
use serde_json::Value;

/// `custom_id`s of the buttons in a rendered pager row, in order.
///
/// Total by construction — the workspace lints deny `expect`/indexing outside
/// `#[test]` fns — so an unexpected shape yields an empty list, which every
/// assertion below treats as a failure.
fn custom_ids(page: usize, total_pages: usize) -> Vec<String> {
    let row = serde_json::to_value(QueuePager::buttons(page, total_pages))
        .unwrap_or(Value::Null);

    row.get("components")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|button| button.get("custom_id"))
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

#[test]
fn single_page_queue_gets_distinct_button_ids() {
    let ids = custom_ids(0, 1);

    assert_eq!(ids.len(), 2, "expected two buttons, got {ids:?}");
    assert_ne!(ids.first(), ids.get(1), "duplicated custom_id: {ids:?}");
}

#[test]
fn every_page_gets_distinct_button_ids() {
    for total_pages in 1..=5 {
        for page in 0..total_pages {
            let ids = custom_ids(page, total_pages);

            assert_eq!(
                ids.len(),
                2,
                "expected two buttons on page {page} of {total_pages}, got {ids:?}"
            );
            assert_ne!(
                ids.first(),
                ids.get(1),
                "duplicated custom_id on page {page} of {total_pages}: {ids:?}"
            );
        }
    }
}

/// `queue_embed` clamps an out-of-range page to the last one, so the buttons
/// have to clamp identically or they page against a section the embed never
/// shows.
#[test]
fn out_of_range_page_is_clamped_to_the_last_page() {
    // Three pages, so the clamped page is 2: "Previous" targets 1, "Next" 3.
    assert_eq!(custom_ids(50, 3), custom_ids(2, 3));
}
