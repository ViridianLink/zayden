//! `/destiny2 builds` shows perk, mod, fragment, stat and artifact emoji as a
//! `#` heading line so they render large. With nothing to show, that line must
//! be left out: a bare `#` renders as a stray hash in Discord.

use destiny2::loadouts::markdown::{emoji_section, with_emoji_row};

#[test]
fn an_emoji_row_follows_its_head() {
    assert_eq!(
        with_emoji_row("**Dead Messenger**", " <:a:1> <:b:2>"),
        "**Dead Messenger**\n# <:a:1> <:b:2>"
    );
}

#[test]
fn an_empty_row_leaves_only_the_head() {
    assert_eq!(with_emoji_row("**Dead Messenger**", ""), "**Dead Messenger**");
    assert_eq!(with_emoji_row("**Dead Messenger**", "   "), "**Dead Messenger**");
}

#[test]
fn a_section_with_emoji_keeps_its_title() {
    assert_eq!(
        emoji_section("### ARTIFACT PERKS", " <:a:1>").as_deref(),
        Some("### ARTIFACT PERKS\n# <:a:1>")
    );
}

#[test]
fn an_empty_section_is_left_out() {
    assert_eq!(emoji_section("### Stats Priority", ""), None);
    assert_eq!(emoji_section("### ARTIFACT PERKS", " "), None);
}
