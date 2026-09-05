//! `text::truncate` is the last thing between a long body and Discord's
//! character limits, so the invariant that matters is that the result never
//! exceeds `limit` — including the ellipsis it appends.

use zayden_core::text::{close_fence, truncate};

const ELLIPSIS: &str = "\n\n_(truncated)_";

#[test]
fn content_within_the_limit_is_returned_unchanged() {
    assert_eq!(truncate("short", 100, ELLIPSIS), "short");
}

#[test]
fn content_exactly_at_the_limit_is_returned_unchanged() {
    let content = "x".repeat(50);

    assert_eq!(truncate(&content, 50, ELLIPSIS), content);
}

#[test]
fn a_truncated_result_stays_within_the_limit() {
    let content = "word ".repeat(500);
    let out = truncate(&content, 100, ELLIPSIS);

    assert!(out.chars().count() <= 100, "{} chars", out.chars().count());
    assert!(out.ends_with(ELLIPSIS), "{out}");
}

#[test]
fn the_cut_prefers_a_paragraph_break_in_the_second_half() {
    let content = format!("{}\n\nrest of the body that will be cut", "a".repeat(60));
    let out = truncate(&content, 80, ELLIPSIS);

    assert_eq!(out, format!("{}{ELLIPSIS}", "a".repeat(60)));
}

/// A break in the first half of the budget would throw away most of the
/// allowance, so the cut falls back to the raw boundary instead.
#[test]
fn an_early_break_is_not_used_as_the_boundary() {
    let content = format!("a\n{}", "b".repeat(200));
    let out = truncate(&content, 100, ELLIPSIS);

    assert!(out.contains("bbb"), "{out}");
}

#[test]
fn a_multibyte_body_is_cut_on_a_character_boundary() {
    let content = "\u{e9}".repeat(300);
    let out = truncate(&content, 50, ELLIPSIS);

    assert!(out.chars().count() <= 50, "{} chars", out.chars().count());
    assert!(out.starts_with('\u{e9}'), "{out}");
}

#[test]
fn an_ellipsis_longer_than_the_limit_does_not_panic() {
    let out = truncate(&"x".repeat(100), 3, ELLIPSIS);

    assert_eq!(out, ELLIPSIS);
}

#[test]
fn an_unbalanced_fence_is_closed() {
    let mut out = String::from("text\n```rust\nlet x = 1;");
    close_fence(&mut out);

    assert!(out.ends_with("\n```"), "{out}");
}

#[test]
fn a_balanced_fence_is_left_alone() {
    let mut out = String::from("```\ncode\n```");
    close_fence(&mut out);

    assert_eq!(out, "```\ncode\n```");
}

#[test]
fn truncating_inside_a_fence_closes_it_before_the_ellipsis() {
    let content = format!("```rust\n{}\n```", "let x = 1;\n".repeat(50));
    let out = truncate(&content, 120, ELLIPSIS);

    assert!(out.matches("```").count().is_multiple_of(2), "{out}");
}
