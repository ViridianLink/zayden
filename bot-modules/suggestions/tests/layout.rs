use serenity::all::{ChannelId, GenericChannelId, MessageId};
use suggestions::layout::{Candidate, candidate, parse_link, thread_name};

const CONFIGURED: ChannelId = ChannelId::new(1_323_301_324_833_689_611);

#[test]
fn reaction_in_configured_text_channel_is_a_text_suggestion() {
    assert_eq!(
        candidate(
            GenericChannelId::new(CONFIGURED.get()),
            MessageId::new(42),
            CONFIGURED
        ),
        Candidate::TextMessage,
    );
}

#[test]
fn reaction_on_a_thread_starter_message_may_be_a_forum_suggestion() {
    assert_eq!(
        candidate(GenericChannelId::new(77), MessageId::new(77), CONFIGURED),
        Candidate::ThreadStarter,
    );
}

#[test]
fn reaction_in_an_unrelated_channel_is_ignored() {
    assert_eq!(
        candidate(
            GenericChannelId::new(1_301_647_410_556_043_364),
            MessageId::new(5),
            CONFIGURED
        ),
        Candidate::Ignore,
    );
}

#[test]
fn parse_link_reads_channel_and_message_segments() {
    assert_eq!(
        parse_link("https://discord.com/channels/1/2/3"),
        Some((ChannelId::new(2), MessageId::new(3))),
    );
    assert_eq!(
        parse_link("https://discord.com/channels/@me/2/3"),
        Some((ChannelId::new(2), MessageId::new(3))),
    );
}

#[test]
fn parse_link_rejects_malformed_urls() {
    assert_eq!(parse_link("https://discord.com/channels/1/2/abc"), None);
    assert_eq!(parse_link("3"), None);
    assert_eq!(parse_link("https://discord.com/channels/1/0/3"), None);
}

#[test]
fn thread_name_uses_first_line_and_fits_discord_limit() {
    assert_eq!(
        thread_name("Ann", "  Add dark mode\nmore detail"),
        "Ann - Add dark mode"
    );
    assert_eq!(thread_name("Ann", "   "), "Ann");
    assert_eq!(thread_name("Ann", &"x".repeat(300)).chars().count(), 100);
}
