//! A settings form may only store channel and role ids that Discord lists for
//! the guild being edited. The bot is in many guilds, so an id from any other
//! one would let a guild admin direct the bot's posts there.
#![cfg(feature = "ssr")]

use dashboard::server::ownership::first_foreign;

const GUILD_IDS: [&str; 3] = ["100", "200", "300"];

#[test]
fn ids_the_guild_lists_pass() {
    assert_eq!(first_foreign(GUILD_IDS, &[100, 300]), None);
}

#[test]
fn nothing_submitted_passes() {
    assert_eq!(first_foreign(GUILD_IDS, &[]), None);
}

#[test]
fn an_id_from_another_guild_is_reported() {
    assert_eq!(first_foreign(GUILD_IDS, &[100, 999, 200]), Some(999));
}

#[test]
fn every_id_is_checked_not_just_the_first() {
    assert_eq!(first_foreign(GUILD_IDS, &[100, 200, 400]), Some(400));
}

#[test]
fn a_guild_with_no_listing_owns_nothing() {
    assert_eq!(first_foreign([], &[100]), Some(100));
}

/// `parse_id` accepts a leading minus; no snowflake is negative, so such an id
/// must not slip through by wrapping onto a listed one.
#[test]
fn a_negative_id_is_foreign() {
    let wrapped = (-1_i64).cast_unsigned().to_string();

    assert_eq!(first_foreign([wrapped.as_str()], &[-1]), Some(-1));
}
