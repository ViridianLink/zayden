use jiff_sqlx::ToSqlx;
use youtube::announce::message_content;
use youtube::store::PendingVideo;

fn pending(channel_title: &str) -> PendingVideo {
    PendingVideo {
        video_id: "abc123".to_owned(),
        channel_id: "UCexample".to_owned(),
        channel_title: channel_title.to_owned(),
        title: "A video".to_owned(),
        published_at: jiff::Timestamp::UNIX_EPOCH.to_sqlx(),
    }
}

#[test]
fn the_message_names_the_channel_and_links_the_video() {
    assert_eq!(
        message_content(&pending("Example Creator")),
        "New video from **Example Creator**\nhttps://www.youtube.com/watch?v=abc123"
    );
}

/// A channel name is creator-controlled text posted into someone else's
/// server, so it must not be able to ping.
#[test]
fn a_channel_name_cannot_mention_everyone() {
    let content = message_content(&pending("@everyone"));

    assert!(!content.contains("@everyone"), "{content}");
}
