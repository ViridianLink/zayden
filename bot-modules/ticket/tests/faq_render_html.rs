//! Wiki.js pages carry raw HTML that Discord renders literally. The tag that
//! prompted this was `<img src="/linuxcontainers.png" class="tab-icon"> 2 ·`,
//! which reached users verbatim.

mod common;
use common::render;

#[test]
fn an_img_tag_is_removed_and_its_text_kept() {
    let out =
        render("<img src=\"/linuxcontainers.png\" class=\"tab-icon\"> 2 \u{b7}");

    assert_eq!(out.trim(), "2 \u{b7}");
}

#[test]
fn inline_tags_are_unwrapped() {
    let out = render("press <kbd>Ctrl</kbd> then <b>Enter</b>");

    assert_eq!(out.trim(), "press Ctrl then Enter");
}

#[test]
fn script_and_style_take_their_contents_with_them() {
    let out = render("before<script>alert('x')</script>after");

    assert_eq!(out.trim(), "beforeafter");
    assert!(!out.contains("alert"), "{out}");
}

#[test]
fn breaks_become_newlines() {
    let out = render("one<br>two<br/>three");

    assert_eq!(out.trim(), "one\ntwo\nthree");
}

#[test]
fn entities_are_decoded() {
    let out = render("Tom &amp; Jerry &lt;3 &#39;quoted&#39; &nbsp;end");

    assert!(out.contains("Tom & Jerry"), "{out}");
    assert!(out.contains("<3"), "{out}");
    assert!(out.contains("'quoted'"), "{out}");
}

/// A comparison is not a tag, and neither is an autolink.
#[test]
fn angle_brackets_that_are_not_tags_survive() {
    let out = render("if a < b and b > c\n\n<https://example.com>");

    assert!(out.contains("a < b"), "{out}");
    assert!(out.contains("<https://example.com>"), "{out}");
}

/// A quoted attribute may contain the character that ends the tag.
#[test]
fn a_bracket_inside_an_attribute_does_not_end_the_tag_early() {
    let out = render("<span title=\"a > b\">kept</span>");

    assert_eq!(out.trim(), "kept");
}

/// HTML in a code sample is the subject of the sample, not markup to strip.
#[test]
fn html_inside_a_fence_is_left_alone() {
    let content = "```html\n<img src=\"/x.png\">\n```";

    assert!(render(content).contains("<img src=\"/x.png\">"));
}

#[test]
fn html_inside_inline_code_is_left_alone() {
    assert!(render("use `<br>` for a break").contains("`<br>`"));
}
