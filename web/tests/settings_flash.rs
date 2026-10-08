//! The flash cookie the settings pages use for their results: signed, one
//! time, never empty, and optionally aimed at one section of the page.

use std::error::Error;

use http_body_util::BodyExt;
use topcoat::Result as ViewResult;
use topcoat::context::Cx;
use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, header, page};
use topcoat::view::{View, view};
use web::flash::{
    self,
    FLASH_COOKIE,
    Flash,
    FlashError,
    FlashKind,
    MAX_MESSAGE_CHARS,
};

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[page("/set/blank")]
async fn set_blank(cx: &Cx) -> ViewResult<impl View> {
    let refused = matches!(
        flash::set(cx, FlashKind::Success, "  "),
        Err(FlashError::EmptyMessage)
    );
    Ok(view! { (if refused { "refused" } else { "set" }) })
}

#[page("/set/section")]
async fn set_section(cx: &Cx) -> ViewResult<impl View> {
    flash::set_in(
        cx,
        FlashKind::Success,
        "Music settings saved.",
        "music-settings",
    )?;
    Ok(view! { "set" })
}

#[page("/take")]
async fn take(cx: &Cx) -> ViewResult<impl View> {
    let shown = flash::take(cx).map_or_else(
        || "none".to_owned(),
        |flash| {
            format!(
                "{:?}|{}|{}",
                flash.kind,
                flash.section.unwrap_or_default(),
                flash.message
            )
        },
    );
    Ok(view! { (shown) })
}

fn app() -> Router {
    Router::builder().page(set_blank).page(set_section).page(take).cookies().build()
}

async fn get(path: &str, cookie: Option<&str>) -> TestResult<Response> {
    let mut request = Request::get(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    Ok(app().handle(request.body(Body::empty())?).await)
}

async fn text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn set_cookie(response: &Response) -> Option<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with(&format!("{FLASH_COOKIE}=")))
        .map(str::to_owned)
}

#[tokio::test]
async fn a_blank_message_is_refused_and_sets_no_cookie() {
    let response = get("/set/blank", None).await.unwrap();

    assert_eq!(set_cookie(&response), None);
    assert_eq!(text(response).await.unwrap(), "refused");
}

#[tokio::test]
async fn a_signed_flash_reaches_its_section_once() {
    let response = get("/set/section", None).await.unwrap();
    let cookie = set_cookie(&response).unwrap();
    assert!(
        cookie.contains("; HttpOnly; SameSite=Lax; Path=/; Max-Age=120"),
        "{cookie}"
    );
    let pair = cookie.split(';').next().unwrap().to_owned();
    let value = pair.split_once('=').unwrap().1;
    assert_ne!(
        value,
        Flash::in_section(
            FlashKind::Success,
            "Music settings saved.",
            "music-settings"
        )
        .encode(),
        "the cookie carries a signature, not the bare encoding"
    );

    let response = get("/take", Some(&pair)).await.unwrap();
    let cleared = set_cookie(&response).unwrap();
    assert!(cleared.starts_with(&format!("{FLASH_COOKIE}=;")), "{cleared}");
    assert!(cleared.contains("Max-Age=0"), "{cleared}");
    assert_eq!(
        text(response).await.unwrap(),
        "Success|music-settings|Music settings saved."
    );
}

#[tokio::test]
async fn an_unsigned_flash_is_cleared_and_not_shown() {
    let forged =
        Flash::new(FlashKind::Error, "Session expired, re-enter your token");
    let response =
        get("/take", Some(&format!("{FLASH_COOKIE}={}", forged.encode())))
            .await
            .unwrap();

    assert!(
        set_cookie(&response)
            .is_some_and(|c| c.starts_with(&format!("{FLASH_COOKIE}=;")))
    );
    assert_eq!(text(response).await.unwrap(), "none");
    assert_eq!(text(get("/take", None).await.unwrap()).await.unwrap(), "none");
}

#[test]
fn a_section_survives_the_encoding_and_a_bad_id_falls_to_the_top() {
    let aimed = Flash::in_section(FlashKind::Success, "Saved.", "ticket-settings");
    assert_eq!(aimed.section.as_deref(), Some("ticket-settings"));
    assert_eq!(Flash::decode(&aimed.encode()), Some(aimed.clone()));
    assert!(aimed.is_for("ticket-settings"));
    assert!(!aimed.is_for("wiki-key"));

    for bad in ["", "Wiki", "a b", "x\"y", "a\nb"] {
        let flash = Flash::in_section(FlashKind::Error, "Not saved.", bad);
        assert_eq!(flash.section, None, "{bad:?}");
        assert_eq!(Flash::decode(&flash.encode()), Some(flash), "{bad:?}");
    }
}

#[test]
fn decoding_refuses_what_set_would_never_write() {
    assert_eq!(Flash::decode(&Flash::new(FlashKind::Success, " ").encode()), None);
    let long = "x".repeat(MAX_MESSAGE_CHARS + 5);
    assert_eq!(
        Flash::new(FlashKind::Error, &long).message.chars().count(),
        MAX_MESSAGE_CHARS
    );
    assert_eq!(Flash::decode("not base64 !"), None);
}
