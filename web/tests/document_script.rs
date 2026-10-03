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
