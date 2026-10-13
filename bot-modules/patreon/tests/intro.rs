use jiff_sqlx::ToSqlx;
use patreon::embeds::{intro_component, intro_content};
use patreon::store::{PatreonAnnounceRow, PendingPost};

fn post(title: &str) -> PendingPost {
    PendingPost {
        post_id: "1".to_owned(),
        campaign_id: "555000".to_owned(),
        title: Some(title.to_owned()),
        url: "https://www.patreon.com/creator/posts/a-post-1".to_owned(),
        content_html: None,
        thumbnail_url: None,
        is_public: false,
        published_at: jiff::Timestamp::UNIX_EPOCH.to_sqlx(),
    }
}

fn row(message: Option<&str>, ping_role_ids: Vec<i64>) -> PatreonAnnounceRow {
    PatreonAnnounceRow {
        guild_id: 1,
        channel_id: 2,
        public_only: false,
        message: message.map(str::to_owned),
        ping_role_ids,
    }
}

#[test]
fn no_message_and_no_roles_posts_only_the_card() {
    assert!(intro_component(&post("A post"), &row(None, Vec::new())).is_none());
}

#[test]
fn roles_alone_are_mentioned_above_the_card() {
    assert_eq!(intro_content(&post("A post"), &row(None, vec![9])), "<@&9>");
}

#[test]
fn the_message_fills_the_post_placeholders() {
    let content = intro_content(
        &post("A post"),
        &row(Some("{roles} {title} ({access}) {url}"), vec![9]),
    );

    assert_eq!(
        content,
        "<@&9> A post (Patrons only) https://www.patreon.com/creator/posts/a-post-1"
    );
}

#[test]
fn a_post_title_cannot_mention_everyone() {
    let content =
        intro_content(&post("@everyone"), &row(Some("{title}"), Vec::new()));

    assert!(!content.contains("@everyone"), "{content}");
}
