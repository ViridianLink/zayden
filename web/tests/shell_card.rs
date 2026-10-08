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
    let gambling = module("gambling", "Gambling & Economy", Some(false), None);
    Ok(view! { module_card(module: &gambling, guild_id: "7") })
}

#[page("/card/on")]
async fn card_on() -> ViewResult<impl View> {
    let music = module("music", "Music", Some(true), None);
    Ok(view! { module_card(module: &music, guild_id: "7") })
}

#[page("/card/settings")]
async fn card_settings() -> ViewResult<impl View> {
    let ai = module("ai", "AI Chat", Some(false), None);
    Ok(view! { module_card(module: &ai, guild_id: "7") })
}

#[page("/card/derived")]
async fn card_derived() -> ViewResult<impl View> {
    let patreon = module("patreon", "Patreon", Some(true), Some("derived"));
    Ok(view! { module_card(module: &patreon, guild_id: "7") })
}

#[page("/card/failed")]
async fn card_failed() -> ViewResult<impl View> {
    let music = module("music", "Music", Some(false), None);
    Ok(view! {
        module_card(module: &music, guild_id: "7", error: Some("Not changed: nope"))
    })
}

async fn render(path: &str) -> TestResult<String> {
    let router = Router::builder()
        .page(card_unknown)
        .page(card_off)
        .page(card_on)
        .page(card_settings)
        .page(card_derived)
        .page(card_failed)
        .runtime()
        .build();
    let response = router.handle(Request::get(path).body(Body::empty())?).await;
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn switch_form(module_id: &str, label: &str, on: bool) -> String {
    let (lamp, text, checked, wanted) = if on {
        ("on", "On", "true", "false")
    } else {
        ("off", "Off", "false", "true")
    };
    format!(
        r#"<form class="rack-form" method="post" action="/guild/7" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="module_id" value="{module_id}"><div class="rack-state"><span class="lamp-status"><span class="lamp lamp-{lamp}" aria-hidden="true"></span><span class="lamp-text" data-pending-text="Saving…">{text}</span></span></div><div class="rack-control"><button type="submit" class="switch" role="switch" aria-checked="{checked}" aria-label="{label} module" name="enabled" value="{wanted}"><span class="switch-thumb" aria-hidden="true"></span></button></div></form>"#
    )
}

fn status(lamp: &str, text: &str, href: &str) -> String {
    format!(
        r#"<div class="rack-state"><span class="lamp-status"><span class="lamp lamp-{lamp}" aria-hidden="true"></span><span class="lamp-text">{text}</span></span></div><div class="rack-control"><a href="{href}" class="rack-link">Manage{CHEVRON_RIGHT}</a></div>"#
    )
}

#[tokio::test]
async fn a_module_without_state_shows_the_not_synced_note_and_no_switch() {
    let html = render("/card/unknown").await.unwrap();

    assert_eq!(
        html,
        concat!(
            r#"<li class="rack-row"><div class="rack-main"><h3 class="rack-name"><a href="/guild/7/music">Music</a></h3><p class="rack-desc">Music description</p></div>"#,
            r#"<div class="rack-state"><span class="lamp-status"><span class="lamp lamp-sync" aria-hidden="true"></span><span class="lamp-text">Not synced</span></span></div><div class="rack-control"></div>"#,
            r#"<p class="rack-note">Not synced yet: this module's state appears once Zayden is in the server and has synced its commands.</p></li>"#,
        )
    );
}

#[tokio::test]
async fn a_command_module_that_is_off_offers_a_switch_that_turns_it_on() {
    let html = render("/card/off").await.unwrap();

    assert_eq!(
        html,
        format!(
            r#"<li class="rack-row"><div class="rack-main"><h3 class="rack-name">Gambling &amp; Economy</h3><p class="rack-desc">Gambling &amp; Economy description</p></div>{}</li>"#,
            switch_form("gambling", "Gambling &amp; Economy", false)
        )
    );
}

#[tokio::test]
async fn a_command_module_that_is_on_offers_a_switch_that_turns_it_off() {
    let html = render("/card/on").await.unwrap();

    assert_eq!(
        html,
        format!(
            r#"<li class="rack-row"><div class="rack-main"><h3 class="rack-name"><a href="/guild/7/music">Music</a></h3><p class="rack-desc">Music description</p></div>{}</li>"#,
            switch_form("music", "Music", true)
        )
    );
}

#[tokio::test]
async fn modules_switched_elsewhere_show_their_state_and_a_link_instead_of_a_switch()
{
    let ai = render("/card/settings").await.unwrap();
    assert!(ai.contains(&status("off", "Off", "/guild/7/ai")), "{ai}");
    assert!(!ai.contains("<form"), "{ai}");
    assert!(!ai.contains(r#"role="switch""#), "{ai}");

    let patreon = render("/card/derived").await.unwrap();
    assert!(patreon.contains(&status("on", "On", "/guild/7/patreon")), "{patreon}");
    assert!(!patreon.contains("<form"), "{patreon}");
    assert!(!patreon.contains("derived"), "{patreon}");
}

#[tokio::test]
async fn a_failed_switch_shows_its_reason_on_the_row_and_keeps_the_stored_state() {
    let html = render("/card/failed").await.unwrap();

    assert!(
        html.ends_with(&format!(
            r#"{}<p class="rack-error" role="alert">Not changed: nope</p></li>"#,
            switch_form("music", "Music", false)
        )),
        "{html}"
    );
}
