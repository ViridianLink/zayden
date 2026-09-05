//! Patreon bodies are HTML and Discord renders it literally, so anything that
//! reaches the message unconverted shows up as raw tags to every reader.

use patreon::content::{BODY_LIMIT, ELLIPSIS, to_discord, to_markdown};

#[test]
fn plain_paragraphs_become_lines() {
    assert_eq!(to_discord("<p>one</p><p>two</p>"), "one\ntwo");
}

#[test]
fn emphasis_becomes_markdown() {
    let out = to_discord(
        "<p><strong>bold</strong> and <em>italic</em> and <u>under</u></p>",
    );

    assert_eq!(out, "**bold** and _italic_ and __under__");
}

#[test]
fn a_link_becomes_a_labelled_markdown_link() {
    let out =
        to_discord(r#"<p>see <a href="https://example.com/x">the notes</a></p>"#);

    assert_eq!(out, "see [the notes](https://example.com/x)");
}

/// `[https://x](https://x)` is noise; Discord renders a bare URL fine.
#[test]
fn a_link_whose_label_is_its_url_renders_bare() {
    let out = to_discord(r#"<a href="https://example.com">https://example.com</a>"#);

    assert_eq!(out, "https://example.com");
}

#[test]
fn list_items_become_bullets() {
    let out = to_discord("<ul><li>one</li><li>two</li></ul>");

    assert_eq!(out, "- one\n- two");
}

#[test]
fn headings_become_discord_headings() {
    assert_eq!(to_discord("<h1>Title</h1><p>body</p>"), "## Title\nbody");
    assert_eq!(to_discord("<h4>Sub</h4><p>body</p>"), "### Sub\nbody");
}

#[test]
fn breaks_and_rules_survive() {
    assert_eq!(to_discord("a<br>b"), "a\nb");
    assert_eq!(to_discord("a<hr>b"), "a\n---\nb");
}

#[test]
fn entities_are_decoded() {
    assert_eq!(to_discord("<p>Tom &amp; Jerry &lt;3</p>"), "Tom & Jerry <3");
}

#[test]
fn script_and_style_contents_are_dropped() {
    let out = to_discord("<p>before</p><script>alert('x')</script><p>after</p>");

    assert_eq!(out, "before\nafter");
    assert!(!out.contains("alert"), "{out}");
}

/// Patreon's lead image is the same asset the thumbnail lookup finds, so an
/// inline `<img>` would only duplicate it.
#[test]
fn images_are_dropped() {
    let out =
        to_discord(r#"<p>text <img src="https://example.com/a.png"> more</p>"#);

    assert!(!out.contains("a.png"), "{out}");
    assert!(out.contains("text"), "{out}");
}

#[test]
fn an_unknown_tag_keeps_its_text() {
    assert_eq!(to_discord("<p>a <span>b</span> c</p>"), "a b c");
}

#[test]
fn runs_of_blank_lines_collapse() {
    let out = to_discord("<div><p>a</p></div><div><div><p>b</p></div></div>");

    assert!(!out.contains("\n\n\n"), "{out:?}");
}

#[test]
fn an_empty_body_yields_an_empty_string() {
    assert_eq!(to_discord(""), "");
    assert_eq!(to_discord("<p></p>"), "");
}

#[test]
fn a_long_body_is_truncated_within_the_limit() {
    let html = format!("<p>{}</p>", "sentence ".repeat(2000));
    let out = to_discord(&html);

    assert!(out.chars().count() <= BODY_LIMIT, "{} chars", out.chars().count());
    assert!(out.ends_with(ELLIPSIS), "no truncation marker: {out:?}");
}

#[test]
fn a_short_body_is_not_marked_truncated() {
    assert!(!to_discord("<p>short</p>").contains(ELLIPSIS));
}

/// `to_markdown` runs before tag stripping, so it must leave the markup it does
/// not handle for `strip` rather than eating it.
#[test]
fn unhandled_markup_is_left_for_the_stripper() {
    let out = to_markdown("<table><tr><td>cell</td></tr></table>");

    assert!(out.contains("cell"), "{out}");
}

#[test]
fn malformed_html_does_not_panic() {
    let out = to_discord("<p>unclosed <strong>bold <a href=\"x\">link</p>");

    assert!(out.contains("unclosed"), "{out}");
}
