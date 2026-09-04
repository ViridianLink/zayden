//! Turning Wiki.js Markdown into something Discord will render.
//!
//! Pages routinely exceed Discord's body budget, Wiki.js emits `{class="..."}`
//! attribute blocks and root-relative links that only resolve on the wiki
//! itself, and a cut landing inside a fenced code block turns everything after
//! it into code.

use ticket::faq::render::{BODY_LIMIT, thumbnail, truncate};

mod common;
use common::{config, render};

#[test]
fn short_content_is_untouched() {
    let content = "# Docker\n\nA guide.";

    assert_eq!(truncate(content, BODY_LIMIT), content);
}

#[test]
fn long_content_is_cut_to_the_limit() {
    let content = "line of text\n".repeat(1000);

    let out = truncate(&content, BODY_LIMIT);

    assert!(out.chars().count() <= BODY_LIMIT, "{} chars", out.chars().count());
    assert!(out.contains("truncated"));
}

/// A cut inside a fence would otherwise leave the marker unbalanced and render
/// every following character as code.
#[test]
fn an_open_code_fence_is_closed() {
    let content = format!("intro\n\n```bash\n{}", "echo hello\n".repeat(1000));

    let out = truncate(&content, BODY_LIMIT);

    assert_eq!(out.matches("```").count() % 2, 0, "unbalanced fence in: {out}");
}

/// `truncate` slices by characters; a byte-indexed cut would panic here.
#[test]
fn multibyte_content_is_cut_on_a_char_boundary() {
    let content = "\u{e9}".repeat(9000);

    let out = truncate(&content, BODY_LIMIT);

    assert!(out.chars().count() <= BODY_LIMIT);
}

#[test]
fn images_and_attribute_blocks_are_stripped() {
    let content = "# ![](/docker.png){class=\"tab-icon\"} What is Docker?";

    let out = render(content);

    assert!(!out.contains("!["), "{out}");
    assert!(!out.contains("{class="), "{out}");
    assert!(out.contains("What is Docker?"), "{out}");
}

/// Root-relative targets resolve against the site root, not the article base:
/// `/docker.png` is an asset, not a page under `/en/`.
#[test]
fn relative_links_become_absolute() {
    let content = "see [the guide](/jellyfin) for more";

    let out = render(content);

    assert!(out.contains("https://wiki.example.com/jellyfin"), "{out}");
}

#[test]
fn absolute_links_are_left_alone() {
    let content = "see [upstream](https://docs.docker.com/engine/) for more";

    let out = render(content);

    assert!(out.contains("https://docs.docker.com/engine/"), "{out}");
}

/// A brace that is not an attribute block is prose, and prose is not eaten.
#[test]
fn prose_braces_survive() {
    let content = "set {{ variable }} in the template";

    let out = render(content);

    assert!(out.contains("{{ variable }}"), "{out}");
}

/// The transforms are span aware: a code sample is not Markdown and must not be
/// rewritten as though it were.
#[test]
fn markdown_syntax_inside_a_fence_is_left_alone() {
    let content = "```markdown\n![alt](/pic.png)\n[link](/page)\n```";

    let out = render(content);

    assert!(out.contains("![alt](/pic.png)"), "{out}");
    assert!(out.contains("[link](/page)"), "{out}");
}

#[test]
fn markdown_syntax_inside_inline_code_is_left_alone() {
    let content = "write `![alt](/pic.png)` to embed it";

    let out = render(content);

    assert!(out.contains("`![alt](/pic.png)`"), "{out}");
}

/// The first image on a page is usually the tab icon, so a thumbnail is chosen
/// by content rather than by position.
#[test]
fn page_chrome_is_not_chosen_as_the_thumbnail() {
    let content =
        "# ![](/docker.png){class=\"tab-icon\"} Docker\n\n![](/screenshot.png)";

    let image = thumbnail(content, &config());

    assert_eq!(
        image.map(|url| url.to_string()),
        Some(String::from("https://wiki.example.com/screenshot.png"))
    );
}

#[test]
fn an_icon_filename_is_not_chosen_as_the_thumbnail() {
    let content = "![](/assets/favicon.png)\n\n![](/diagram.png)";

    let image = thumbnail(content, &config());

    assert_eq!(
        image.map(|url| url.to_string()),
        Some(String::from("https://wiki.example.com/diagram.png"))
    );
}

/// An image in a table cell is a row marker, not the subject of the page.
#[test]
fn a_table_image_is_not_chosen_as_the_thumbnail() {
    let content = "| ![](/row.png) | yes |\n| --- | --- |\n\n![](/hero.png)";

    let image = thumbnail(content, &config());

    assert_eq!(
        image.map(|url| url.to_string()),
        Some(String::from("https://wiki.example.com/hero.png"))
    );
}

#[test]
fn content_without_a_usable_image_yields_none() {
    assert!(thumbnail("plain text", &config()).is_none());
    assert!(thumbnail("![](/logo.svg)", &config()).is_none());
}
