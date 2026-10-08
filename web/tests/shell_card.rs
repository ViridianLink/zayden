use std::error::Error;

use http_body_util::BodyExt;
use topcoat::Result as ViewResult;
use topcoat::router::request::Request;
use topcoat::router::{Body, Router, page};
use topcoat::runtime::RouterBuilderRuntimeExt;
use topcoat::view::{View, view};
use web::guild::dto::ModuleView;
use web::shell::module_card;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const LOCK: &str = "This module is switched on from its own settings page \u{2014} use Configure below.";
const ICON_OPEN: &str = r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">"#;
const CHEVRON_RIGHT: &str = r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 18 6-6-6-6"/></svg>"#;

fn module(
    id: &str,
    label: &str,
    enabled: Option<bool>,
    locked: Option<&str>,
) -> ModuleView {
    ModuleView {
        id: id.to_owned(),
        label: label.to_owned(),
        description: format!("{label} description"),
        enabled,
        locked: locked.map(str::to_owned),
    }
}

#[page("/card/unknown")]
async fn card_unknown() -> ViewResult<impl View> {
    let music = module("music", "Music", None, None);
    Ok(view! { module_card(module: &music, guild_id: "7") })
}

#[page("/card/off")]
async fn card_off() -> ViewResult<impl View> {
    let ai = module("ai", "AI Chat", Some(false), None);
    Ok(view! { module_card(module: &ai, guild_id: "7") })
}

#[page("/card/on")]
async fn card_on() -> ViewResult<impl View> {
    let gambling = module("gambling", "Gambling & Economy", Some(true), None);
    Ok(view! { module_card(module: &gambling, guild_id: "7") })
}

#[page("/card/locked")]
async fn card_locked() -> ViewResult<impl View> {
    let patreon = module("patreon", "Patreon", Some(true), Some(LOCK));
    Ok(view! { module_card(module: &patreon, guild_id: "7") })
}

#[page("/card/failed")]
async fn card_failed() -> ViewResult<impl View> {
    let ai = module("ai", "AI Chat", Some(false), None);
    Ok(view! {
        module_card(
            module: &ai,
            guild_id: "7",
            error: Some("error running server function: nope")
        )
    })
}

async fn render(path: &str) -> TestResult<String> {
    let router = Router::builder()
        .page(card_unknown)
        .page(card_off)
        .page(card_on)
        .page(card_locked)
        .page(card_failed)
        .runtime()
        .build();
    let response = router.handle(Request::get(path).body(Body::empty())?).await;
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn head(id_tint: &str) -> String {
    format!(
        r#"<div class="module-card"><div class="module-card-head"><div class="module-icon" style="--tint: {id_tint}">{ICON_OPEN}"#
    )
}

#[tokio::test]
async fn an_unknown_module_cannot_be_switched() {
    let html = render("/card/unknown").await.unwrap();

    assert!(html.starts_with(&head("#a78bfa")), "{html}");
    assert!(html.contains(
        r#"<button class="toggle" aria-label="Toggle module" aria-pressed="mixed" disabled=""></button></div><div class="module-name">Music</div><p class="module-desc">Music description</p><p class="module-locked">Zayden hasn't set this module up for this server yet, so it can't be changed right now.</p><div class="module-card-foot"><span class="module-status">Unknown</span><a href="/guild/7/music" class="module-configure">Configure"#
    ), "{html}");
    assert!(
        html.ends_with(&format!("Configure{CHEVRON_RIGHT}</a></div></div>")),
        "{html}"
    );
    assert!(!html.contains("<form"));
}

#[tokio::test]
async fn a_switched_off_module_posts_a_switch_on() {
    let html = render("/card/off").await.unwrap();

    assert!(html.starts_with(&head("#22d3ee")), "{html}");
    assert!(html.contains(
        r#"<button class="toggle" aria-label="Toggle module" aria-pressed="false" form="module-toggle-ai" name="enabled" value="true"></button>"#
    ), "{html}");
    assert!(html.contains(
        r#"<div class="module-card-foot"><span class="module-status">Disabled</span><a href="/guild/7/ai" class="module-configure">"#
    ), "{html}");
    assert!(html.ends_with(
        r#"</div><form id="module-toggle-ai" method="post" action="/guild/7"><input type="hidden" name="guild" value="7"><input type="hidden" name="module_id" value="ai"></form></div>"#
    ), "{html}");
    assert!(!html.contains("module-locked"));
    assert!(!html.contains("module-error"));
}

#[tokio::test]
async fn a_switched_on_module_posts_a_switch_off() {
    let html = render("/card/on").await.unwrap();

    assert!(html.starts_with(&head("#f472b6")), "{html}");
    assert!(html.contains(
        r#"<button class="toggle toggle-on" aria-label="Toggle module" aria-pressed="true" form="module-toggle-gambling" name="enabled" value="false"></button>"#
    ), "{html}");
    assert!(html.contains(
        r#"<div class="module-card-foot"><span class="module-status on">Enabled</span></div><form id="module-toggle-gambling""#
    ), "{html}");
}

#[tokio::test]
async fn a_module_switched_on_elsewhere_is_locked() {
    let html = render("/card/locked").await.unwrap();

    assert!(html.contains(
        r#"<button class="toggle toggle-on" aria-label="Toggle module" aria-pressed="true" disabled=""></button>"#
    ), "{html}");
    assert!(
        html.contains(&format!(r#"<p class="module-locked">{LOCK}</p>"#)),
        "{html}"
    );
    assert!(html.contains(
        r#"<span class="module-status on">Enabled</span><a href="/guild/7/patreon" class="module-configure">"#
    ), "{html}");
    assert!(!html.contains("<form"));
}

#[tokio::test]
async fn a_failed_toggle_shows_its_error_on_the_card() {
    let html = render("/card/failed").await.unwrap();

    assert!(html.contains(
        r#"<p class="module-desc">AI Chat description</p><p class="module-error">error running server function: nope</p><div class="module-card-foot"><span class="module-status failed">Not saved</span>"#
    ), "{html}");
    assert!(
        html.contains(r#"aria-pressed="false" form="module-toggle-ai""#),
        "{html}"
    );
}
