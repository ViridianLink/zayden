//! Truncating on a paragraph boundary can end a response halfway through a
//! procedure. Cutting on a heading keeps whatever is shown complete.

use ticket::faq::render::{anchor, best_match, fit, split_sections};

const PAGE: &str = "\
Intro text.

# Install

Run the installer.

## Backups

Copy the config directory.

## Restore

Put it back.";

#[test]
fn a_page_splits_into_its_headings() {
    let sections = split_sections(PAGE);

    let titles = sections.iter().map(|s| s.title.as_str()).collect::<Vec<_>>();

    assert_eq!(titles, ["", "Install", "Backups", "Restore"]);
}

#[test]
fn text_before_the_first_heading_is_its_own_section() {
    let sections = split_sections(PAGE);

    assert_eq!(sections.first().map(|s| s.body.trim()), Some("Intro text."));
}

/// A `#` in a shell sample is a comment, not a heading.
#[test]
fn a_hash_inside_a_fence_is_not_a_heading() {
    let content = "# Real\n\n```bash\n# not a heading\necho hi\n```";

    let sections = split_sections(content);

    assert_eq!(sections.len(), 1);
    assert!(
        sections.first().is_some_and(|s| s.body.contains("# not a heading")),
        "{sections:?}"
    );
}

#[test]
fn anchors_match_the_wiki_slug_format() {
    assert_eq!(anchor("Backups"), "backups");
    assert_eq!(anchor("Set up the API key"), "set-up-the-api-key");
    assert_eq!(anchor("Docker & Podman"), "docker-podman");
}

#[test]
fn the_best_match_is_the_section_the_question_names() {
    let sections = split_sections(PAGE);

    let found = best_match(&sections, "where are the backups stored");

    assert_eq!(found.map(|s| s.title.as_str()), Some("Backups"));
}

/// A heading hit outranks a body mention.
#[test]
fn a_heading_hit_beats_a_body_mention() {
    let sections = split_sections(
        "# Overview\n\nBackups are covered later.\n\n# Backups\n\nHow to.",
    );

    let found = best_match(&sections, "backups");

    assert_eq!(found.map(|s| s.title.as_str()), Some("Backups"));
}

/// The point of the exercise: the cut lands on a heading, so no section is shown
/// half-finished.
#[test]
fn the_cut_lands_on_a_section_boundary() {
    let sections = split_sections(PAGE);

    let out = fit(&sections, 90);

    assert!(out.chars().count() <= 90, "{} chars: {out}", out.chars().count());
    assert!(out.contains("truncated"), "{out}");
    assert!(!out.contains("Copy the config directory"), "{out}");
    assert!(
        out.contains("Run the installer.") || out.contains("Intro text."),
        "{out}"
    );
}

#[test]
fn everything_fitting_is_returned_whole() {
    let sections = split_sections(PAGE);

    let out = fit(&sections, 4000);

    assert!(!out.contains("truncated"), "{out}");
    assert!(out.contains("Put it back."), "{out}");
}

/// Discord has no `####`, so deeper headings flatten to bold rather than
/// rendering their hashes.
#[test]
fn deep_headings_flatten_to_bold() {
    let sections = split_sections("#### Deep\n\nbody");

    assert!(
        sections.first().is_some_and(|s| s.render().starts_with("**Deep**")),
        "{sections:?}"
    );
}
