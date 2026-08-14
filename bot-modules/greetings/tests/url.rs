//! Offline tests for the two pure functions in `greetings`: the URL gate that
//! decides what may be stored, and the placeholder expansion that turns a
//! stored template into message content.
//!
//! Neither touches Postgres, so these run without `DATABASE_URL`.

use greetings::{GreetingKind, GreetingsError, render, validate_url};
use serenity::all::UserId;

const TARGET: UserId = UserId::new(100);
const INVOKER: UserId = UserId::new(200);

/// `validate_url` is the only thing standing between an admin's form input and
/// both an `<img src>` on the dashboard and an embed handed to Discord, so the
/// rejections matter more than the acceptances.
mod validate {
    use super::{GreetingsError, validate_url};

    #[test]
    fn accepts_https_and_returns_it_trimmed() {
        let url = validate_url("  https://example.com/sunrise.gif  ")
            .expect("a plain https link is valid");
        assert_eq!(url, "https://example.com/sunrise.gif");
    }

    #[test]
    fn accepts_query_strings_and_ports() {
        for url in [
            "https://example.com:8443/a.png",
            "https://cdn.example.com/a.gif?width=400&h=300",
            "https://example.com/a%20b.png#frag",
        ] {
            validate_url(url).unwrap_or_else(|e| {
                panic!("{url} should be accepted, got {e:?}");
            });
        }
    }

    /// Requiring `https://` is what makes `javascript:` and `data:` impossible,
    /// which is the reason the check exists at all.
    #[test]
    fn rejects_every_non_https_scheme() {
        for url in [
            "http://example.com/a.png",
            "javascript:alert(1)",
            "data:image/png;base64,iVBORw0KGgo=",
            "file:///etc/passwd",
            "//example.com/a.png",
            "example.com/a.png",
            "HTTPS://example.com/a.png",
        ] {
            let err = validate_url(url)
                .expect_err("only a literal https:// prefix may pass");
            assert!(
                matches!(err, GreetingsError::InvalidUrl(_)),
                "{url} should be InvalidUrl, got {err:?}"
            );
        }
    }

    #[test]
    fn rejects_empty_and_scheme_only() {
        for url in ["", "   ", "https://", "  https://  "] {
            let err = validate_url(url).expect_err("there is no host to fetch");
            assert!(matches!(err, GreetingsError::InvalidUrl(_)), "{url:?}");
        }
    }

    /// Whitespace and control characters would let a link break out of the
    /// attribute it is rendered into on the dashboard.
    #[test]
    fn rejects_embedded_whitespace_and_control_characters() {
        for url in [
            "https://example.com/a b.png",
            "https://example.com/a\tb.png",
            "https://example.com/a\nb.png",
            "https://example.com/a\u{0}b.png",
        ] {
            let err = validate_url(url).expect_err("must not contain raw controls");
            assert!(matches!(err, GreetingsError::InvalidUrl(_)), "{url:?}");
        }
    }

    #[test]
    fn rejects_over_long_urls() {
        let long = format!("https://example.com/{}", "a".repeat(2048));
        let err = validate_url(&long).expect_err("2048 is the cap");
        assert!(matches!(err, GreetingsError::InvalidUrl(_)), "{err:?}");
    }
}

#[test]
fn user_expands_to_the_target_mention() {
    assert_eq!(
        render("Good morning {user}!", TARGET, INVOKER),
        "Good morning <@100>!"
    );
}

#[test]
fn author_expands_to_the_invoker_mention() {
    assert_eq!(render("from {author}", TARGET, INVOKER), "from <@200>");
}

#[test]
fn both_placeholders_expand_independently() {
    assert_eq!(
        render("{author} greets {user}", TARGET, INVOKER),
        "<@200> greets <@100>"
    );
}

#[test]
fn every_occurrence_is_replaced() {
    assert_eq!(
        render("{user} {user} {author} {author}", TARGET, INVOKER),
        "<@100> <@100> <@200> <@200>"
    );
}

/// With no `user` argument the command passes the invoker as both, and the two
/// placeholders must then agree rather than one of them going stale.
#[test]
fn target_and_invoker_may_be_the_same_person() {
    assert_eq!(render("{author} -> {user}", INVOKER, INVOKER), "<@200> -> <@200>");
}

#[test]
fn text_without_placeholders_is_untouched() {
    assert_eq!(
        render("Good night, everyone", TARGET, INVOKER),
        "Good night, everyone"
    );
}

/// Unknown tokens are left verbatim rather than erroring or being stripped, so
/// a typo shows up in the message where the admin can see and fix it.
#[test]
fn unknown_tokens_are_left_verbatim() {
    assert_eq!(
        render("{server} {users} {USER} {user}", TARGET, INVOKER),
        "{server} {users} {USER} <@100>"
    );
}

/// The two-pass implementation is only safe because a rendered mention cannot
/// contain a token for the next pass to find.
#[test]
fn a_rendered_mention_is_not_rescanned() {
    assert_eq!(render("{user}{author}", TARGET, INVOKER), "<@100><@200>");
}

#[test]
fn default_messages_use_the_user_placeholder() {
    for kind in [GreetingKind::Morning, GreetingKind::Night] {
        let rendered = render(kind.default_message(), TARGET, INVOKER);
        assert!(
            rendered.contains("<@100>"),
            "{kind} default must greet the target, got {rendered:?}"
        );
    }
}

#[test]
fn kind_round_trips_through_its_stored_string() {
    for kind in [GreetingKind::Morning, GreetingKind::Night] {
        let parsed = GreetingKind::parse(kind.as_str()).expect("round trip");
        assert_eq!(parsed, kind);
    }

    let err = GreetingKind::parse("afternoon").expect_err("not a greeting");
    assert!(matches!(err, GreetingsError::UnknownKind(_)), "{err:?}");
}
