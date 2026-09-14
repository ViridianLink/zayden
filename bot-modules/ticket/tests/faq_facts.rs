//! The verbatim details a merge or discard must not lose.
//!
//! The model is told to keep every command, path, version and link, but a
//! prompt is not a control, least of all for details newer than its training
//! data that it is inclined to "correct". `facts::missing` is the check in code;
//! these tests pin what counts as a detail and that reformatting is not loss.

use ticket::faq::facts::{literals, missing};

#[test]
fn a_code_span_is_a_detail() {
    assert!(literals("Set `PUID=1000` in the compose file.").contains("PUID=1000"));
}

#[test]
fn fenced_code_lines_are_details_but_the_fence_is_not() {
    let found = literals("```yaml\nports:\n  - 8080:80\n```");

    assert!(found.contains("ports:"), "{found:?}");
    assert!(found.contains("- 8080:80"), "{found:?}");
    assert!(!found.iter().any(|literal| literal.contains("```")), "{found:?}");
}

#[test]
fn a_url_is_a_detail_without_its_trailing_punctuation() {
    let found = literals("See https://wiki.example.com/radarr/setup.");

    assert!(found.contains("https://wiki.example.com/radarr/setup"), "{found:?}");
}

#[test]
fn a_version_is_a_detail() {
    assert!(literals("Fixed in Sonarr v4.0.15 and later.").contains("v4.0.15"));
}

#[test]
fn plain_prose_holds_no_details() {
    assert!(literals("Restart the container and try again.").is_empty());
}

#[test]
fn a_dropped_command_is_reported() {
    let required =
        literals("Run `docker compose pull` then `docker compose up -d`.");

    assert_eq!(missing(&required, "Run `docker compose pull` and restart."), [
        "docker compose up -d"
    ]);
}

#[test]
fn moving_a_detail_into_a_code_block_is_not_loss() {
    let required = literals("Run `docker   compose up -d`.");

    let lost = missing(&required, "```sh\ndocker compose up -d\n```");

    assert!(lost.is_empty(), "{lost:?}");
}
