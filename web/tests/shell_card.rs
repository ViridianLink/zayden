use std::error::Error;

use http_body_util::BodyExt;
use topcoat::Result as ViewResult;
use topcoat::router::request::Request;
use topcoat::router::{Body, Router, page};
use topcoat::runtime::RouterBuilderRuntimeExt;
use topcoat::view::{View, view};
use web::guild::dto::ModuleView;
use web::shell::module_card;

mod canonical;

use canonical::canonical;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const ERROR_ALERT: &str = "grid w-full grid-cols-[0_1fr] items-start gap-y-1 rounded-lg border bg-background px-4 py-3 text-sm has-[>svg]:grid-cols-[1rem_1fr] has-[>svg]:gap-x-3 [&amp;>svg]:size-4 [&amp;>svg]:translate-y-0.5 border-destructive/50 text-destructive";
const LAMP_STATUS: &str = "inline-flex items-center gap-2 whitespace-nowrap";
const LAMP_ON: &str = "lamp-on inline-block w-[10px] h-[10px] shrink-0 box-border rounded-(--radius-full)";
const LAMP_OFF: &str = "inline-block w-[10px] h-[10px] shrink-0 box-border rounded-(--radius-full) border-2 border-solid border-line bg-transparent";
const LAMP_SYNC: &str = "lamp-sync inline-block w-[10px] h-[10px] shrink-0 box-border rounded-(--radius-full)";
const RACK_ROW: &str = "grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-x-4 min-h-(--row-h) py-3 px-4 border-t border-solid border-t-rule first:[border-top:0] max-md:grid-cols-[minmax(0,1fr)_auto] max-md:gap-y-2 max-md:min-h-[48px]";
const RACK_MAIN: &str = "min-w-0";
const RACK_NAME: &str = "m-0 text-[15px] font-semibold";
const RACK_NAME_LINK: &str = "inline-flex items-center min-h-(--target-min) max-md:min-h-(--row-h) text-text no-underline decoration-current hover:underline";
const RACK_DESC: &str = "m-0 text-[13px] text-text-2";
const RACK_STATE: &str = "min-w-[7rem] max-md:col-[1] max-md:row-[2]";
const RACK_CONTROL: &str = "flex items-center justify-end min-w-[48px] max-md:col-[2] max-md:row-[1/span_2]";
const RACK_LINK: &str =
    "inline-flex items-center gap-1 min-h-(--target-min) text-text font-medium";
const RACK_NOTE: &str = "col-span-full m-0 mt-2 text-[13px] text-caution";
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
        (LAMP_ON, "On", "true", "false")
    } else {
        (LAMP_OFF, "Off", "false", "true")
    };
    format!(
        r#"<form class="contents" method="post" action="/guild/7" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="module_id" value="{module_id}"><div class="{RACK_STATE}"><span class="{LAMP_STATUS}"><span class="{lamp}" aria-hidden="true"></span><span class="lamp-text" data-pending-text="Saving…">{text}</span></span></div><div class="{RACK_CONTROL}"><button type="submit" class="switch" role="switch" aria-checked="{checked}" aria-label="{label} module" name="enabled" value="{wanted}"><span class="switch-thumb" aria-hidden="true"></span></button></div></form>"#
    )
}

fn status(lamp: &str, text: &str, href: &str) -> String {
    format!(
        r#"<div class="{RACK_STATE}"><span class="{LAMP_STATUS}"><span class="{lamp}" aria-hidden="true"></span><span class="lamp-text">{text}</span></span></div><div class="{RACK_CONTROL}"><a href="{href}" class="{RACK_LINK}" data-topcoat-link="intent">Manage{CHEVRON_RIGHT}</a></div>"#
    )
}

#[tokio::test]
async fn a_module_without_state_shows_the_not_synced_note_and_no_switch() {
    let html = render("/card/unknown").await.unwrap();

    assert_eq!(
        canonical(&html),
        canonical(&format!(
            r#"<li class="{RACK_ROW}"><div class="{RACK_MAIN}"><h3 class="{RACK_NAME}"><a href="/guild/7/music" class="{RACK_NAME_LINK}" data-topcoat-link="intent">Music</a></h3><p class="{RACK_DESC}">Music description</p></div><div class="{RACK_STATE}"><span class="{LAMP_STATUS}"><span class="{LAMP_SYNC}" aria-hidden="true"></span><span class="lamp-text">Not synced</span></span></div><div class="{RACK_CONTROL}"></div><p class="{RACK_NOTE}">Not synced yet: this module's state appears once Zayden is in the server and has synced its commands.</p></li>"#
        ))
    );
}

#[tokio::test]
async fn a_command_module_that_is_off_offers_a_switch_that_turns_it_on() {
    let html = render("/card/off").await.unwrap();

    assert_eq!(
        html,
        format!(
            r#"<li class="{RACK_ROW}"><div class="{RACK_MAIN}"><h3 class="{RACK_NAME}">Gambling &amp; Economy</h3><p class="{RACK_DESC}">Gambling &amp; Economy description</p></div>{}</li>"#,
            switch_form("gambling", "Gambling &amp; Economy", false)
        )
    );
}

#[tokio::test]
async fn a_command_module_that_is_on_offers_a_switch_that_turns_it_off() {
    let html = render("/card/on").await.unwrap();

    assert_eq!(
        canonical(&html),
        canonical(&format!(
            r#"<li class="{RACK_ROW}"><div class="{RACK_MAIN}"><h3 class="{RACK_NAME}"><a href="/guild/7/music" class="{RACK_NAME_LINK}" data-topcoat-link="intent">Music</a></h3><p class="{RACK_DESC}">Music description</p></div>{}</li>"#,
            switch_form("music", "Music", true)
        ))
    );
}

#[tokio::test]
async fn modules_switched_elsewhere_show_their_state_and_a_link_instead_of_a_switch()
{
    let ai = render("/card/settings").await.unwrap();
    assert!(
        canonical(&ai).contains(&canonical(&status(LAMP_OFF, "Off", "/guild/7/ai"))),
        "{ai}"
    );
    assert!(!ai.contains("<form"), "{ai}");
    assert!(!ai.contains(r#"role="switch""#), "{ai}");

    let patreon = render("/card/derived").await.unwrap();
    assert!(
        canonical(&patreon).contains(&canonical(&status(
            LAMP_ON,
            "On",
            "/guild/7/patreon"
        ))),
        "{patreon}"
    );
    assert!(!patreon.contains("<form"), "{patreon}");
    assert!(!patreon.contains("derived"), "{patreon}");
}

#[tokio::test]
async fn a_failed_switch_shows_its_reason_on_the_row_and_keeps_the_stored_state() {
    let html = render("/card/failed").await.unwrap();

    assert!(
        canonical(&html).ends_with(&canonical(&format!(
            r#"{}<div class="{ERROR_ALERT} col-span-full m-0 mt-2" role="alert"><p class="col-start-2 font-medium tracking-tight">Not changed: nope</p></div></li>"#,
            switch_form("music", "Music", false)
        ))),
        "{html}"
    );
}
