//! A patrons-only post's body is what patrons pay for; the announcement in a
//! Discord channel must carry only its title and link.

use jiff_sqlx::ToSqlx;
use patreon::embeds::post_component;
use patreon::store::PendingPost;

const BODY: &str = "Exclusive walkthrough text";

fn post(is_public: bool) -> PendingPost {
    PendingPost {
        post_id: "1".to_owned(),
        campaign_id: "555000".to_owned(),
        title: Some("A post".to_owned()),
        url: "https://www.patreon.com/creator/posts/a-post-1".to_owned(),
        content_html: Some(format!("<p>{BODY}</p>")),
        thumbnail_url: None,
        is_public,
        published_at: jiff::Timestamp::constant(1_788_220_800, 0).to_sqlx(),
    }
}

fn render(post: &PendingPost) -> String {
    serde_json::to_string(&post_component(post)).unwrap_or_default()
}

#[test]
fn a_public_post_carries_its_body() {
    let rendered = render(&post(true));

    assert!(rendered.contains(BODY), "{rendered}");
}

#[test]
fn a_patrons_only_post_withholds_its_body() {
    let rendered = render(&post(false));

    assert!(!rendered.contains(BODY), "{rendered}");
    assert!(rendered.contains("A post"), "{rendered}");
    assert!(rendered.contains("https://www.patreon.com/creator/posts/a-post-1"));
}
