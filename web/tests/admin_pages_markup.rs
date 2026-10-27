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
use web::app::admin::servers::servers_page;
use web::auth::AuthError;
use web::components::guild_grid::GuildCard;

mod canonical;
mod support;

use canonical::canonical;

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

const HEADER: &str = r#"<div class="page"><div class="page-header"><div><h1>All bot servers</h1><p class="page-lead">Every server Zayden is in. Operator access ignores your own permissions in them.</p></div></div>"#;
const INPUT: &str = "h-9 max-md:h-11 w-full min-w-0 rounded-lg border border-input bg-popover px-3 text-sm transition-colors outline-none placeholder:text-muted-foreground file:mr-3 file:h-full file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 aria-invalid:border-destructive aria-invalid:focus-visible:ring-destructive focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50";
fn tools() -> String {
    format!(
        r#"<div class="operator-tools"><div class="field-row"><label class="field-label" for="server-filter">Filter by name</label><input class="{INPUT} grow basis-64" id="server-filter" type="search" autocomplete="off" value=""></div><div class="field-row operator-jump"><label class="field-label" for="server-jump">Go to server ID</label><input class="{INPUT} [&amp;]:w-52 font-mono" id="server-jump" type="text" inputmode="numeric" autocomplete="off" value=""><button type="button" class="{secondary}" disabled="">Go</button><a class="{secondary}" href="" hidden="">Go</a></div></div>"#,
        secondary = support::btn("secondary")
    )
}
const ERROR_ALERT: &str = r#"<div class="grid w-full grid-cols-[0_1fr] items-start gap-y-1 rounded-lg border bg-background px-4 py-3 text-sm has-[>svg]:grid-cols-[1rem_1fr] has-[>svg]:gap-x-3 [&amp;>svg]:size-4 [&amp;>svg]:translate-y-0.5 border-destructive/50 text-destructive error"><p class="col-start-2 font-medium tracking-tight">"#;
const TABLE: &str = "w-full caption-bottom border-collapse border-border text-sm [:not(:first-child)>&amp;]:border-t [:not(:last-child)>&amp;]:border-b data-table";
const CAPTION: &str = "mt-4 text-sm text-muted-foreground visually-hidden";
const THEAD: &str = "bg-foreground/5 [&amp;_tr]:border-b";
const TBODY: &str = "[&amp;_tr:last-child]:border-0";
const ROW: &str = "border-b border-border transition-colors hover:bg-foreground/5";
const TH: &str = "p-3 text-left align-middle font-medium whitespace-nowrap text-muted-foreground first:pl-6 last:pr-6";
const TD: &str = "p-3 align-middle whitespace-nowrap first:pl-6 last:pr-6";

fn table_head() -> String {
    format!(
        r#"<div class="w-full overflow-x-auto" data-slot="table"><table class="{TABLE}" role="table"><caption class="{CAPTION}">Servers Zayden is in</caption><thead class="{THEAD}" role="rowgroup"><tr class="{ROW}" role="row"><th class="{TH}" scope="col" role="columnheader">Server</th><th class="{TH}" scope="col" role="columnheader">Server ID</th></tr></thead><tbody class="{TBODY}" role="rowgroup">"#
    )
}

fn server_row(id: &str, name: &str) -> String {
    format!(
        r#"<tr role="row"><th class="{TH}" scope="row" role="rowheader" data-label="Server">{name}</th><td class="{TD}" role="cell" data-label="Server ID"><span class="mono">{id}</span></td></tr>"#
    )
}

#[tokio::test]
async fn the_list_has_labelled_tools_an_announced_count_and_a_table_row_per_server()
{
    let html = render("/t/list").await.unwrap();

    assert_eq!(
        canonical(&html),
        canonical(&format!(
            "{HEADER}{tools}{}{table_head}{}{}{}",
            concat!(
                r#"<div role="status"><p class="empty" hidden="">No server matches that name.</p>"#,
                r#"<p class="operator-count">2 of 2 servers</p></div><div>"#,
            ),
            server_row(
                "7",
                r#"<a href="/guild/7"><span class="guild-icon placeholder" aria-hidden="true">Z</span><span class="guild-name">Zayden's Server</span></a>"#,
            ),
            server_row(
                "9",
                r#"<a href="/guild/9"><img src="https://cdn.discordapp.com/icons/9/a_0123456789abcdef0123456789abcdef.png?size=64" alt="" class="guild-icon"><span class="guild-name">-Mark-'s</span></a>"#,
            ),
            "</tbody></table></div></div></div>",
            tools = tools(),
            table_head = table_head(),
        ))
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
        html.matches(r#"<tr role="row" data-topcoat-bind:hidden="#).count(),
        2
    );
    assert!(html.contains("<div data-topcoat-bind:hidden="), "{html}");
    for marker in [r#"<p class="empty""#, r#"<p class="operator-count""#] {
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
        canonical(&html),
        canonical(&format!(
            "{HEADER}{tools}{}{table_head}{}",
            concat!(
                r#"<div role="status"><p class="empty">No server matches that name.</p>"#,
                r#"<p class="operator-count" hidden="">0 of 0 servers</p></div><div hidden="">"#,
            ),
            "</tbody></table></div></div></div>",
            tools = tools(),
            table_head = table_head(),
        ))
    );
}

#[tokio::test]
async fn a_refused_list_shows_the_access_line_without_the_tools() {
    for path in ["/t/denied", "/t/signed-out"] {
        let html = render(path).await.unwrap();

        assert_eq!(
            html,
            format!(
                "{HEADER}{ERROR_ALERT}{}",
                "Operator access is required to list every server.</p></div></div>"
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
            "{HEADER}{ERROR_ALERT}{}",
            "Couldn't load the server list: error running server function: 429 Too Many Requests</p></div></div>"
        )
    );
}
