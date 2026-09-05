//! Reading the pages a ticket links to.
//!
//! Triage used to see the URL as an opaque token, so a reporter who pasted
//! their whole log got asked for the log. Extraction has to survive the shapes
//! Discord actually produces - markdown links, `<>`-suppressed embeds, a URL
//! ending a sentence - because a URL that fails to parse is a page that goes
//! unread and a question that gets asked anyway.

use ticket::faq::linked::{excerpt, readable, urls};

const LIMIT: usize = 3;

#[test]
fn a_bare_url_is_found() {
    let found = urls("it dies on startup, log: https://paste.ee/r/abc123", LIMIT);

    assert_eq!(found.iter().map(ToString::to_string).collect::<Vec<_>>(), [
        "https://paste.ee/r/abc123"
    ]);
}

#[test]
fn markdown_and_angle_brackets_do_not_become_part_of_the_url() {
    let found = urls(
        "see [the log](https://paste.ee/r/abc) and <https://example.com/issue/7>",
        LIMIT,
    );

    assert_eq!(found.iter().map(ToString::to_string).collect::<Vec<_>>(), [
        "https://paste.ee/r/abc",
        "https://example.com/issue/7"
    ]);
}

#[test]
fn a_url_ending_a_sentence_keeps_its_path_but_drops_the_full_stop() {
    let found = urls("the crash is at https://example.com/logs/latest.txt.", LIMIT);

    assert_eq!(found.iter().map(ToString::to_string).collect::<Vec<_>>(), [
        "https://example.com/logs/latest.txt"
    ]);
}

#[test]
fn the_same_page_linked_twice_is_only_read_once() {
    let found = urls("https://example.com/a and again https://example.com/a", LIMIT);

    assert_eq!(found.len(), 1);
}

#[test]
fn a_wall_of_links_stops_at_the_limit() {
    let content = (0..10)
        .map(|i| format!("https://example.com/{i}"))
        .collect::<Vec<_>>()
        .join(" ");

    assert_eq!(urls(&content, LIMIT).len(), LIMIT);
}

#[test]
fn non_web_schemes_are_left_alone() {
    let found = urls("mailto:someone@example.com ftp://example.com/x", LIMIT)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn page_text_survives_the_markup_around_it() {
    let body = "<html><head><title>Ignored</title><style>p { color: red }</style>\
                <script>alert('no')</script></head><body>\
                <h1>Server will not boot</h1>\
                <p>The port was already bound.</p>\
                <pre>Address already in use</pre>\
                </body></html>";

    let text = readable(body);

    assert_eq!(
        text,
        "Server will not boot\nThe port was already bound.\nAddress already in use"
    );
}

#[test]
fn scripts_and_styles_never_reach_the_model() {
    let text = readable("<body><script>secret_token='abc'</script><p>hi</p></body>");

    assert_eq!(text, "hi");
    assert!(!text.contains("secret_token"));
}

#[test]
fn an_excerpt_stops_on_a_line_boundary_rather_than_mid_line() {
    let text = "first line\nsecond line\nthird line";

    assert_eq!(excerpt(text, 22).as_deref(), Some("first line\nsecond line"));
}

#[test]
fn a_page_with_nothing_readable_yields_no_excerpt() {
    assert_eq!(excerpt("   \n\n  \n", 100), None);
    assert_eq!(readable("<body><script>x=1</script></body>"), String::new());
}
