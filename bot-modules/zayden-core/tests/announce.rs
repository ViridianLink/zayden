use zayden_core::announce::{escape, render, role_mentions};

#[test]
fn every_known_placeholder_is_filled() {
    assert_eq!(
        render("{title} by {channel}: {url}", &[], &[
            ("title", "Patch"),
            ("channel", "Dev"),
            ("url", "https://x.example"),
        ]),
        "Patch by Dev: https://x.example"
    );
}

#[test]
fn unknown_and_unclosed_placeholders_are_left_as_written() {
    assert_eq!(render("{nope} {title", &[], &[("title", "T")]), "{nope} {title");
}

#[test]
fn a_substituted_value_is_not_scanned_for_placeholders() {
    assert_eq!(
        render("{title} {url}", &[5], &[("title", "{roles} {url}"), ("url", "u")]),
        "<@&5> {roles} {url} u"
    );
}

#[test]
fn roles_go_where_the_template_puts_them() {
    assert_eq!(render("New video {roles}", &[1, 2], &[]), "New video <@&1> <@&2>");
}

#[test]
fn roles_lead_a_template_without_the_placeholder() {
    assert_eq!(render("New video", &[1], &[]), "<@&1> New video");
}

#[test]
fn an_empty_roles_placeholder_leaves_no_stray_whitespace() {
    assert_eq!(render("{roles}\nNew video", &[], &[]), "New video");
    assert_eq!(render("", &[], &[]), "");
}

#[test]
fn role_mentions_skip_invalid_ids() {
    assert_eq!(role_mentions(&[0, -1, 3]), "<@&3>");
}

#[test]
fn escaped_text_cannot_mention_everyone_or_format() {
    let escaped = escape("@everyone **bold**");

    assert!(!escaped.contains("@everyone"), "{escaped}");
    assert!(!escaped.contains("**"), "{escaped}");
}
