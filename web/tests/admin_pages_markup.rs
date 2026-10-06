//! The operator server list's markup for each state, rendered from fixed
//! data: the signed-in list needs the bot's guild list from Discord, which
//! no in-process case can reach.

use std::error::Error;

use http_body_util::BodyExt;
use topcoat::Result as ViewResult;
use topcoat::router::request::Request;
use topcoat::router::{Body, Router, RouterBuilder, page};
use topcoat::runtime::RouterBuilderRuntimeExt;
use topcoat::view::{View, view};
use web::admin::AdminError;
use web::admin::pages::servers::servers_page;
use web::auth::AuthError;
use web::components::guild_grid::GuildCard;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

fn card(id: &str, name: &str, icon: Option<&str>) -> GuildCard {
    GuildCard { id: id.into(), name: name.into(), icon: icon.map(Into::into) }
}

#[page("/t/list")]
async fn listed() -> ViewResult<impl View> {
    Ok(view! {
        servers_page(
            guilds: Ok(
                vec![
                    card("7", "Zayden's Server", None),
                    card("9", "-Mark-'s", Some("a_0123456789abcdef0123456789abcdef")),
                ],
            )
        )
    })
}

#[page("/t/none")]
async fn none() -> ViewResult<impl View> {
    Ok(view! { servers_page(guilds: Ok(Vec::new())) })
}

#[page("/t/denied")]
async fn denied() -> ViewResult<impl View> {
    Ok(view! { servers_page(guilds: Err(AdminError::Auth(AuthError::Forbidden))) })
}

#[page("/t/signed-out")]
async fn signed_out() -> ViewResult<impl View> {
    Ok(
        view! { servers_page(guilds: Err(AdminError::Auth(AuthError::Unauthenticated))) },
    )
}

#[page("/t/failed")]
async fn failed() -> ViewResult<impl View> {
    Ok(view! {
        servers_page(guilds: Err(AdminError::Discord("429 Too Many Requests".into())))
    })
}

fn builder() -> RouterBuilder {
    Router::builder()
        .page(listed)
        .page(none)
        .page(denied)
        .page(signed_out)
        .page(failed)
}

async fn render_raw(path: &str) -> TestResult<String> {
    let router = builder().runtime().build();
    let response = router.handle(Request::get(path).body(Body::empty())?).await;
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

/// Rendered HTML without comments and runtime binding attributes.
async fn render(path: &str) -> TestResult<String> {
    Ok(normalize(&render_raw(path).await?))
}

fn normalize(html: &str) -> String {
    let html = strip_between(html, "<!--", |rest| {
        rest.split_once("-->").map_or("", |(_, tail)| tail)
    });
    strip_between(&html, " data-topcoat-", |rest| {
        rest.split_once("=\"")
            .and_then(|(_, value)| value.split_once('"'))
            .map_or("", |(_, tail)| tail)
    })
}

fn strip_between(html: &str, marker: &str, skip: impl Fn(&str) -> &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((before, after)) = rest.split_once(marker) {
        out.push_str(before);
        rest = skip(after);
    }
    out.push_str(rest);
    out
}

const HEADER: &str = r#"<div class="page"><div class="page-header"><div><h1>All Servers</h1><p class="page-lead">Every server Zayden is in. Operator access ignores your own permissions in them.</p></div></div>"#;
const TOOLS: &str = r#"<div class="operator-tools"><input class="input" type="search" placeholder="Filter by name" aria-label="Filter servers by name" value=""><div class="operator-jump"><input class="input" type="text" inputmode="numeric" placeholder="Go to server ID" aria-label="Go to server ID" value=""><button type="button" class="btn btn-secondary" disabled="">Go</button><a class="btn btn-secondary" href="" hidden="">Go</a></div></div>"#;

#[tokio::test]
async fn the_list_has_the_tools_the_count_and_every_card() {
    let html = render("/t/list").await.unwrap();

    assert_eq!(
        html,
        format!(
            "{HEADER}{TOOLS}{}",
            concat!(
                r#"<p class="empty" hidden="">No server matches that name.</p>"#,
                r#"<p class="operator-count">2 of 2 servers</p>"#,
                r#"<div class="guild-grid">"#,
                r#"<a href="/guild/7" class="guild-card"><span class="guild-icon placeholder">Z</span><div class="guild-card-body"><div class="guild-name">Zayden's Server</div><div class="guild-card-hint">Manage &rarr;</div></div></a>"#,
                r#"<a href="/guild/9" class="guild-card"><img src="https://cdn.discordapp.com/icons/9/a_0123456789abcdef0123456789abcdef.png?size=64" alt="" class="guild-icon"><div class="guild-card-body"><div class="guild-name">-Mark-'s</div><div class="guild-card-hint">Manage &rarr;</div></div></a>"#,
                r#"</div></div>"#,
            )
            .replace("&rarr;", "\u{2192}")
        )
    );
}

#[tokio::test]
async fn the_filter_and_the_go_link_are_bound_to_what_is_typed() {
    let html = render_raw("/t/list").await.unwrap();

    assert_eq!(html.matches("data-topcoat-on:input=").count(), 2);
    assert_eq!(html.matches("data-topcoat-bind:value=").count(), 2);
    assert_eq!(html.matches("data-topcoat-bind:href=").count(), 1);
    assert!(html.contains("BigInt("));
    assert_eq!(
        html.matches(r#"class="guild-card" data-topcoat-bind:hidden="#).count(),
        2
    );
    for marker in [
        r#"<p class="empty""#,
        r#"<p class="operator-count""#,
        r#"<div class="guild-grid""#,
    ] {
        let tag = html.split(marker).nth(1).unwrap_or_default();
        assert!(
            tag.split('>')
                .next()
                .unwrap_or_default()
                .contains("data-topcoat-bind:hidden="),
            "{marker}"
        );
    }
}

#[tokio::test]
async fn no_servers_reads_as_no_match() {
    let html = render("/t/none").await.unwrap();

    assert_eq!(
        html,
        format!(
            "{HEADER}{TOOLS}{}",
            concat!(
                r#"<p class="empty">No server matches that name.</p>"#,
                r#"<p class="operator-count" hidden="">0 of 0 servers</p>"#,
                r#"<div class="guild-grid" hidden=""></div></div>"#,
            )
        )
    );
}

#[tokio::test]
async fn a_refused_list_shows_the_access_line_without_the_tools() {
    for path in ["/t/denied", "/t/signed-out"] {
        let html = render(path).await.unwrap();

        assert_eq!(
            html,
            format!(
                "{HEADER}{}",
                r#"<p class="error">Operator access is required to list every server.</p></div>"#
            ),
            "{path}"
        );
    }
}

#[tokio::test]
async fn a_failed_list_shows_the_prefixed_reason() {
    let html = render("/t/failed").await.unwrap();

    assert_eq!(
        html,
        format!(
            "{HEADER}{}",
            r#"<p class="error">Couldn't load the server list: error running server function: 429 Too Many Requests</p></div>"#
        )
    );
}
