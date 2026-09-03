//! Redacting a ticket transcript before it reaches the model provider.
//!
//! Prompt instructions are not a control: an LLM asked to hide credentials
//! still repeats them often enough to matter, and a generated FAQ article is
//! published to everyone who can run "/ticket faq ask". Everything sensitive is
//! therefore removed in code, before the request is built.
//!
//! Removing any single pattern from `PATTERNS` in `faq/scrub.rs` fails the test
//! named after it; removing the ordering guarantee (credential URLs first)
//! fails `a_credential_url_keeps_its_scheme_and_host`.

use ticket::faq::scrub::redact;

const REDACTED: &str = "[redacted]";

#[test]
fn a_discord_mention_is_removed() {
    assert_eq!(
        redact("ping <@123456789012345678> please"),
        "ping [redacted] please"
    );
}

#[test]
fn a_bare_snowflake_is_removed() {
    assert_eq!(redact("user 987654321098765432"), "user [redacted]");
}

#[test]
fn an_ipv4_address_is_removed() {
    assert_eq!(redact("connect to 192.168.1.44:8080"), "connect to [redacted]:8080");
}

#[test]
fn an_ipv6_address_is_removed() {
    assert!(!redact("bind [2001:db8::1]:443").contains("2001"));
    assert!(!redact("host fe80:0:0:0:1:2:3:4 down").contains("fe80"));
}

#[test]
fn an_email_address_is_removed() {
    assert_eq!(redact("mail admin@example.com now"), "mail [redacted] now");
}

#[test]
fn a_credential_url_keeps_its_scheme_and_host() {
    let scrubbed = redact("postgres://user:hunter2@db.internal/app");

    assert_eq!(scrubbed, "postgres://[redacted]@db.internal/app");
}

#[test]
fn an_api_key_is_removed() {
    assert!(!redact("key sk-abcdefghijklmnopqrstuvwxyz01").contains("abcdef"));
    assert!(!redact("token xoxb-1234567890abcdefghijkl").contains("abcdef"));
}

#[test]
fn a_jwt_is_removed() {
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dBjftJeZ4CVPmB92K";

    assert_eq!(redact(jwt), REDACTED);
}

#[test]
fn a_long_hex_run_is_removed() {
    let hash = "d41d8cd98f00b204e9800998ecf8427e";

    assert_eq!(redact(&format!("hash {hash}")), "hash [redacted]");
}

#[test]
fn ordinary_prose_is_untouched() {
    let text = "Radarr returned a 502 after the reverse proxy restarted.";

    assert_eq!(redact(text), text);
}

#[test]
fn a_command_in_a_code_block_survives() {
    let text = "```\ndocker compose restart radarr\n```";

    assert_eq!(redact(text), text);
}

#[test]
fn a_short_hex_colour_survives() {
    let text = "set the accent to #0099ff";

    assert_eq!(redact(text), text);
}
