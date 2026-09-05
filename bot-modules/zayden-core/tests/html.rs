//! `html::strip` flattens untrusted markup to text. These pin the behaviour
//! consumers rely on directly; `ticket`'s `faq_render_html.rs` covers the same
//! ground through its full render pipeline, where fences and inline code are
//! protected before `strip` ever sees them.

use zayden_core::html::strip;

#[test]
fn a_void_tag_is_removed_and_its_text_kept() {
    let out =
        strip("<img src=\"/linuxcontainers.png\" class=\"tab-icon\"> 2 \u{b7}");

    assert_eq!(out.trim(), "2 \u{b7}");
}

#[test]
fn inline_tags_are_unwrapped() {
    assert_eq!(
        strip("press <kbd>Ctrl</kbd> then <b>Enter</b>"),
        "press Ctrl then Enter"
    );
}

#[test]
fn script_and_style_take_their_contents_with_them() {
    let out = strip("before<script>alert('x')</script>after<style>a{}</style>");

    assert_eq!(out, "beforeafter");
    assert!(!out.contains("alert"), "{out}");
}

#[test]
fn breaks_become_newlines_and_closing_blocks_end_the_line() {
    assert_eq!(strip("one<br>two<br/>three"), "one\ntwo\nthree");
    assert_eq!(strip("<p>one</p><p>two</p>"), "one\ntwo\n");
    assert_eq!(strip("<ul><li>a</li><li>b</li></ul>"), "a\nb\n");
}

#[test]
fn a_horizontal_rule_becomes_a_markdown_rule() {
    assert_eq!(strip("a<hr>b"), "a\n---\nb");
}

#[test]
fn entities_are_decoded() {
    let out = strip("Tom &amp; Jerry &lt;3 &#39;quoted&#39; &#x2022; &nbsp;end");

    assert!(out.contains("Tom & Jerry"), "{out}");
    assert!(out.contains("<3"), "{out}");
    assert!(out.contains("'quoted'"), "{out}");
    assert!(out.contains('\u{2022}'), "{out}");
}

/// An unknown or malformed entity is data, not markup.
#[test]
fn an_unrecognised_entity_survives_intact() {
    assert_eq!(
        strip("Q&A and &notanentity; and &#xZZ;"),
        "Q&A and &notanentity; and &#xZZ;"
    );
}

/// A comparison is not a tag, and neither is an autolink.
#[test]
fn angle_brackets_that_are_not_tags_survive() {
    let out = strip("if a < b and b > c <https://example.com>");

    assert!(out.contains("a < b"), "{out}");
    assert!(out.contains("<https://example.com>"), "{out}");
}

/// A quoted attribute may contain the character that ends the tag.
#[test]
fn a_bracket_inside_an_attribute_does_not_end_the_tag_early() {
    assert_eq!(strip("<span title=\"a > b\">kept</span>"), "kept");
}

#[test]
fn an_unclosed_discard_element_swallows_the_remainder() {
    assert_eq!(strip("before<script>alert('x')"), "before");
}
