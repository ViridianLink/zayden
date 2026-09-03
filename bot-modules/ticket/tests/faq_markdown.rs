//! Turning Wiki.js Markdown into something Discord will render.
//!
//! Three things bite here, and each has a test below: pages routinely exceed
//! Discord's 4096-character description budget (8 of a 40-page sample did),
//! Wiki.js emits `{class="..."}` attribute blocks and root-relative links that
//! only resolve on the wiki itself, and a cut that lands inside a fenced code
//! block turns the rest of the embed into code.

use ticket::faq::markdown::{
    DESCRIPTION_LIMIT,
    for_discord,
    take_first_image,
    truncate,
};
use ticket::wiki::WikiConfig;
use zayden_app::config::{FaqSettingsRow, SettingsRow};

fn config() -> Option<WikiConfig> {
    let mut row = FaqSettingsRow::empty(1);
    row.enabled = true;
    row.wiki_url = Some(String::from("https://wiki.example.com"));

    WikiConfig::from_settings(&row).ok().flatten()
}

#[test]
fn short_content_is_untouched() {
    let content = "# Docker\n\nA guide.";

    assert_eq!(truncate(content, DESCRIPTION_LIMIT), content);
}

#[test]
fn long_content_is_cut_to_the_limit() {
    let content = "line of text\n".repeat(1000);

    let out = truncate(&content, DESCRIPTION_LIMIT);

    assert!(
        out.chars().count() <= DESCRIPTION_LIMIT,
        "{} chars",
        out.chars().count()
    );
    assert!(out.contains("truncated"));
}

/// A cut inside a fence would otherwise leave the marker unbalanced and render
/// every following character as code.
#[test]
fn an_open_code_fence_is_closed() {
    let content = format!("intro\n\n```bash\n{}", "echo hello\n".repeat(1000));

    let out = truncate(&content, DESCRIPTION_LIMIT);

    assert_eq!(out.matches("```").count() % 2, 0, "unbalanced fence in: {out}");
}

/// `truncate` slices by characters; a byte-indexed cut would panic here.
#[test]
fn multibyte_content_is_cut_on_a_char_boundary() {
    let content = "\u{e9}".repeat(9000);

    let out = truncate(&content, DESCRIPTION_LIMIT);

    assert!(out.chars().count() <= DESCRIPTION_LIMIT);
}

#[test]
fn images_and_attribute_blocks_are_stripped() {
    let content = "# ![](/docker.png){class=\"tab-icon\"} What is Docker?";

    let out = for_discord(content, &config().expect("test config builds"));

    assert!(!out.contains("!["), "{out}");
    assert!(!out.contains("{class="), "{out}");
    assert!(out.contains("What is Docker?"), "{out}");
}

/// Root-relative targets resolve against the site root, not the article base:
/// `/docker.png` is an asset, not a page under `/en/`.
#[test]
fn relative_links_become_absolute() {
    let content = "see [the guide](/jellyfin) for more";

    let out = for_discord(content, &config().expect("test config builds"));

    assert!(out.contains("https://wiki.example.com/jellyfin"), "{out}");
}

#[test]
fn absolute_links_are_left_alone() {
    let content = "see [upstream](https://docs.docker.com/engine/) for more";

    let out = for_discord(content, &config().expect("test config builds"));

    assert!(out.contains("https://docs.docker.com/engine/"), "{out}");
}

/// Discord ignores `![]()` in a description, so the first image is hoisted to
/// `CreateEmbed::image` instead of being dropped on the floor.
#[test]
fn the_first_image_is_recovered_for_the_embed() {
    let content =
        "# ![](/docker.png){class=\"tab-icon\"} Docker\n\n![](/second.png)";

    let image = take_first_image(content, &config().expect("test config builds"));

    assert_eq!(
        image.map(|url| url.to_string()),
        Some(String::from("https://wiki.example.com/docker.png"))
    );
}

#[test]
fn content_without_an_image_yields_none() {
    assert!(
        take_first_image("plain text", &config().expect("test config builds"))
            .is_none()
    );
}
