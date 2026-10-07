use std::error::Error;
use std::path::PathBuf;

use http_body_util::BodyExt;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Method, Router, StatusCode};
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::router;

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const SCRIPT_FILE: &str = "pending-submit-0123456789abcdef.js";
const SCRIPT_SOURCE: &str = include_str!("../assets/pending-submit.js");

fn bundle_dir() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-document-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("tailwind-0123456789abcdef.css"), "")?;
    std::fs::write(dir.join("topcoat-0123456789abcdef.js"), "")?;
    std::fs::write(dir.join(SCRIPT_FILE), SCRIPT_SOURCE)?;
    std::fs::write(
        dir.join("manifest.toml"),
        format!(
            "version = 1\n\n\
             [[assets]]\nid = {}\nfile = \"tailwind-0123456789abcdef.css\"\nhash = \"0\"\ncontent_type = \"text/css\"\n\n\
             [[assets]]\nid = {}\nfile = \"topcoat-0123456789abcdef.js\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n\n\
             [[assets]]\nid = {}\nfile = \"{SCRIPT_FILE}\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n",
            STYLESHEET.id().as_u64(),
            topcoat::runtime::SCRIPT.id().as_u64(),
            PENDING_SUBMIT.id().as_u64(),
        ),
    )?;
    Ok(dir)
}

fn app() -> TestResult<Router> {
    let bundle = AssetBundle::load_dir(bundle_dir()?)?;
    Ok(router(Router::builder().assets(bundle)))
}

async fn send(path: &str) -> TestResult<Response> {
    let request =
        Request::builder().method(Method::GET).uri(path).body(Body::empty())?;
    Ok(app()?.handle(request).await)
}

async fn body_text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

#[tokio::test]
async fn rendered_page_loads_pending_submit_script_once() {
    let html = body_text(send("/does-not-exist").await.unwrap()).await.unwrap();
    let tag = format!(
        "<script type=\"module\" src=\"/_topcoat/assets/{SCRIPT_FILE}\"></script>"
    );
    assert_eq!(html.matches(&tag).count(), 1);
    assert!(html.find(&tag) < html.find("<title>"));
}

#[tokio::test]
async fn bundled_script_body_is_served_from_the_asset_route() {
    let response = send(&format!("/_topcoat/assets/{SCRIPT_FILE}")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await.unwrap(), SCRIPT_SOURCE);
}

#[test]
fn pending_submit_script_targets_opted_in_forms_and_label_attribute() {
    assert!(SCRIPT_SOURCE.contains("form[data-pending]"));
    assert!(SCRIPT_SOURCE.contains("dataset.pendingLabel"));
    assert!(SCRIPT_SOURCE.contains("\"pageshow\""));
}

#[test]
fn pending_submit_listener_defers_disabling_until_after_the_entry_list() {
    let listener = SCRIPT_SOURCE
        .split("document.addEventListener(\"submit\"")
        .nth(1)
        .and_then(|rest| rest.split("setTimeout(").next())
        .unwrap();
    assert!(!listener.contains("disabled"));
    assert!(!listener.contains("setBusy("));
    assert!(SCRIPT_SOURCE.contains("disabledByScript.delete(button)"));
}

fn listener(event: &str) -> Option<&'static str> {
    SCRIPT_SOURCE
        .split(&format!("document.addEventListener(\"{event}\""))
        .nth(1)
        .and_then(|rest| rest.split("\n});").next())
}

#[test]
fn leaving_with_another_dirty_form_asks_before_the_page_goes_busy() {
    assert_eq!(SCRIPT_SOURCE.matches("addEventListener(\"submit\"").count(), 1);
    assert_eq!(SCRIPT_SOURCE.matches("addEventListener(\"click\"").count(), 1);
    assert!(
        SCRIPT_SOURCE.contains("form !== except && form !== sent && isDirty(form)")
    );
    assert!(SCRIPT_SOURCE.contains("window.confirm(LEAVE_PROMPT)"));

    let submit = listener("submit").unwrap();
    let asked = submit.find("confirmLeave(form)").unwrap();
    assert!(asked < submit.find("startProgress()").unwrap());
    assert!(asked < submit.find("setAttribute(\"aria-busy\", \"true\")").unwrap());

    let click = listener("click").unwrap();
    let asked = click.find("confirmLeave(null)").unwrap();
    assert!(asked < click.find("startProgress()").unwrap());
    assert!(asked < click.find("hidePopover()").unwrap());
}

#[test]
fn a_cancelled_navigation_releases_its_approval_and_progress() {
    let release = SCRIPT_SOURCE
        .split("function release()")
        .nth(1)
        .and_then(|rest| rest.split("\n}").next())
        .unwrap();
    assert!(release.contains("leaveApproved = false"));
    assert!(release.contains("stopProgress()"));
    assert!(listener("submit").unwrap().contains("release();"));
    assert!(
        listener("click").unwrap().contains("event.defaultPrevented && release()")
    );
    let unload = SCRIPT_SOURCE
        .split("addEventListener(\"beforeunload\"")
        .nth(1)
        .and_then(|rest| rest.split("\n});").next())
        .unwrap();
    assert!(unload.contains("leaveApproved = false"));
    assert!(unload.contains("dirtyBesides(null)"));
}

#[test]
fn a_hide_only_popover_button_gets_no_expanded_state() {
    assert!(
        SCRIPT_SOURCE
            .contains("invoker.getAttribute(\"popovertargetaction\") !== \"hide\"")
    );
}

#[test]
fn server_rendered_results_are_announced_after_load() {
    assert!(SCRIPT_SOURCE.contains(
        "all(\"[role=status] > [data-flash]\")) note.replaceWith(note.cloneNode(true))"
    ));
}

#[test]
fn the_shared_script_carries_no_banner_comments() {
    assert!(!SCRIPT_SOURCE.contains("/*"));
}
