//! Offline tests for the filename extension a greeting image is uploaded
//! under.
//!
//! Discord picks the inline renderer from the attachment's extension, so this
//! is what decides whether the image shows up in the message or lands as a
//! download link. It is also the only part of the filename influenced by the
//! remote, which makes it the part worth pinning: the function must never
//! return anything but one of its own literals.

use greetings::GreetingsError;
use greetings::attachment::extension_for;

const URL: &str = "https://example.com/sunrise";

/// A content type that names an image wins over whatever the link says, since
/// it describes the bytes actually served.
#[test]
fn a_recognised_content_type_decides_the_extension() {
    for (mime, expected) in [
        ("image/gif", "gif"),
        ("image/png", "png"),
        ("image/apng", "png"),
        ("image/jpeg", "jpg"),
        ("image/jpg", "jpg"),
        ("image/webp", "webp"),
        ("image/avif", "avif"),
    ] {
        let extension = extension_for(Some(mime), "https://example.com/a.png")
            .unwrap_or_else(|e| panic!("{mime} should be usable, got {e:?}"));
        assert_eq!(extension, expected, "for {mime}");
    }
}

#[test]
fn content_type_parameters_and_case_are_ignored() {
    for mime in ["IMAGE/GIF", "image/gif; charset=binary", "  image/gif  "] {
        let extension = extension_for(Some(mime), URL)
            .unwrap_or_else(|e| panic!("{mime:?} should be usable, got {e:?}"));
        assert_eq!(extension, "gif", "for {mime:?}");
    }
}

/// CDNs routinely serve perfectly good images as an opaque binary type, and
/// some send no type at all, so the link is what decides in both cases.
#[test]
fn an_opaque_or_absent_content_type_falls_back_to_the_link() {
    for mime in [None, Some("application/octet-stream"), Some("binary/octet-stream")]
    {
        let extension = extension_for(mime, "https://example.com/sunrise.GIF")
            .unwrap_or_else(|e| panic!("{mime:?} should fall back, got {e:?}"));
        assert_eq!(extension, "gif", "for {mime:?}");
    }
}

#[test]
fn the_link_extension_survives_query_strings_and_fragments() {
    for url in [
        "https://cdn.example.com/a.webp?width=400&h=300",
        "https://cdn.example.com/a.webp#frag",
        "https://cdn.example.com/path.d/a.webp",
    ] {
        let extension = extension_for(None, url)
            .unwrap_or_else(|e| panic!("{url} should be usable, got {e:?}"));
        assert_eq!(extension, "webp", "for {url}");
    }
}

/// Neither source names a format, so the attachment still needs some
/// extension - without one Discord renders a download link instead of the
/// image.
#[test]
fn an_unknown_link_extension_defaults_to_png() {
    for url in [
        "https://example.com/sunrise",
        "https://example.com/sunrise.bin",
        "https://example.com/",
        "https://example.com/a.gif/redirect",
    ] {
        let extension = extension_for(None, url)
            .unwrap_or_else(|e| panic!("{url} should default, got {e:?}"));
        assert_eq!(extension, "png", "for {url}");
    }
}

/// A content type naming something other than an inline-renderable image means
/// the link does not point at one - an HTML error page, a video, or an SVG
/// Discord will not render. Attaching those would be worse than the embed this
/// replaced, so the caller drops the image instead.
#[test]
fn a_non_image_content_type_is_rejected_even_for_an_image_link() {
    for mime in [
        "text/html",
        "text/html; charset=utf-8",
        "application/json",
        "video/mp4",
        "image/svg+xml",
        "image/tiff",
    ] {
        let err = extension_for(Some(mime), "https://example.com/a.gif")
            .expect_err("the link is not what was served");
        assert!(matches!(err, GreetingsError::ImageUnusable(_)), "{mime}: {err:?}");
    }
}
