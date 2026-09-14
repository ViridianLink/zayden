//! Which attachments the triage hands to the vision model.

use ticket::faq::screenshots::is_readable;

const SMALL: u32 = 200 * 1024;

#[test]
fn common_screenshot_formats_are_read() {
    assert!(is_readable(Some("image/png"), "error.png", SMALL));
    assert!(is_readable(Some("image/jpeg"), "photo.jpg", SMALL));
    assert!(is_readable(Some("image/webp"), "clip.webp", SMALL));
}

#[test]
fn content_type_parameters_do_not_hide_the_format() {
    assert!(is_readable(Some("image/PNG; charset=binary"), "error", SMALL));
}

/// A log uploaded as a file is text for the linked-page reader, not an image.
#[test]
fn non_images_are_skipped() {
    assert!(!is_readable(Some("text/plain"), "latest.log", SMALL));
    assert!(!is_readable(Some("video/mp4"), "repro.mp4", SMALL));
}

/// Animated GIFs and SVGs are rejected by several providers outright.
#[test]
fn formats_providers_reject_are_skipped() {
    assert!(!is_readable(Some("image/gif"), "spin.gif", SMALL));
    assert!(!is_readable(Some("image/svg+xml"), "logo.svg", SMALL));
}

#[test]
fn the_extension_decides_when_discord_sends_no_content_type() {
    assert!(is_readable(None, "Screenshot 2026-09-15.PNG", SMALL));
    assert!(!is_readable(None, "crash-report.txt", SMALL));
    assert!(!is_readable(None, "no_extension", SMALL));
}

#[test]
fn oversized_images_are_skipped() {
    assert!(!is_readable(Some("image/png"), "huge.png", 21 * 1024 * 1024));
}
