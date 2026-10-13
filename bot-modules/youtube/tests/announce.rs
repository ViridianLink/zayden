use jiff_sqlx::ToSqlx;
use youtube::announce::message_content;
use youtube::store::{PendingVideo, YoutubeAnnounceRow};

fn pending(channel_title: &str) -> PendingVideo {
    PendingVideo {
        video_id: "abc123".to_owned(),
        channel_id: "UCexample".to_owned(),
        channel_title: channel_title.to_owned(),
        title: "A video".to_owned(),
        published_at: jiff::Timestamp::UNIX_EPOCH.to_sqlx(),
    }
}

fn row(message: Option<&str>, ping_role_ids: Vec<i64>) -> YoutubeAnnounceRow {
    YoutubeAnnounceRow {
        guild_id: 1,
        channel_id: 2,
        message: message.map(str::to_owned),
        ping_role_ids,
    }
}

#[test]
fn the_message_names_the_channel_and_links_the_video() {
    assert_eq!(
        message_content(&pending("Example Creator"), &row(None, Vec::new())),
        "New video from **Example Creator**\nhttps://www.youtube.com/watch?v=abc123"
    );
}

/// A channel name is creator-controlled text posted into someone else's
/// server, so it must not be able to ping.
#[test]
fn a_channel_name_cannot_mention_everyone() {
    let content = message_content(&pending("@everyone"), &row(None, Vec::new()));

    assert!(!content.contains("@everyone"), "{content}");
}

#[test]
fn a_custom_message_fills_every_placeholder() {
    let content = message_content(
        &pending("Example Creator"),
        &row(Some("{roles} {title} from {channel}\n{url}"), vec![42]),
    );

    assert_eq!(
        content,
        "<@&42> A video from Example Creator\nhttps://www.youtube.com/watch?v=abc123"
    );
}

#[test]
fn roles_lead_the_default_message() {
    let content = message_content(&pending("Example Creator"), &row(None, vec![42]));

    assert!(content.starts_with("<@&42> New video from"), "{content}");
}
