//! The settings rows of the redirect map: every old `/guild/{id}/settings/*`
//! address answers 308 to the page that holds it now, query kept, and a form
//! posted to an old address answers 303 to that page with a "not saved"
//! flash. Provider callback targets are covered in `providers_routes.rs`.

use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use http_body_util::BodyExt;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Method, Router, StatusCode, header};
use web::components::brand::LOGO;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::flash::{FLASH_COOKIE, Flash, FlashKind};
use web::router;
use web::settings::NOT_SAVED;

const STYLESHEET_FILE: &str = "tailwind-0123456789abcdef.css";
const RUNTIME_FILE: &str = "topcoat-0123456789abcdef.js";
const PENDING_FILE: &str = "pending-submit-0123456789abcdef.js";
const LOGO_FILE: &str = "logo-0123456789abcdef.png";
const FORM: &str = "application/x-www-form-urlencoded";

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Every legacy settings slug and the page it lives on now, for guild 7.
const MOVED: [(&str, &str); 14] = [
    ("general", "/guild/7/settings"),
    ("ai", "/guild/7/ai"),
    ("family", "/guild/7/family"),
    ("honeypot", "/guild/7/honeypot"),
    ("lfg", "/guild/7/lfg"),
    ("music", "/guild/7/music"),
    ("patreon", "/guild/7/patreon"),
    ("youtube", "/guild/7/youtube"),
    ("temp-voice", "/guild/7/temp-voice"),
    ("support", "/guild/7/support"),
    ("greetings", "/guild/7/greetings"),
    ("levels", "/guild/7/levels"),
    ("reaction-roles", "/guild/7/reaction-roles"),
    ("bogus", "/guild/7/settings"),
];

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-redirects-settings-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    for file in [STYLESHEET_FILE, RUNTIME_FILE, PENDING_FILE, LOGO_FILE] {
        std::fs::write(dir.join(file), "")?;
    }
    std::fs::write(
        dir.join("manifest.toml"),
        format!(
            "version = 1\n\n\
             [[assets]]\nid = {}\nfile = \"{STYLESHEET_FILE}\"\nhash = \"0\"\ncontent_type = \"text/css\"\n\n\
             [[assets]]\nid = {}\nfile = \"{RUNTIME_FILE}\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n\n\
             [[assets]]\nid = {}\nfile = \"{PENDING_FILE}\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n\n\
             [[assets]]\nid = {}\nfile = \"{LOGO_FILE}\"\nhash = \"0\"\ncontent_type = \"image/png\"\n",
            STYLESHEET.id().as_u64(),
            topcoat::runtime::SCRIPT.id().as_u64(),
            PENDING_SUBMIT.id().as_u64(),
            LOGO.id().as_u64(),
        ),
    )?;
    Ok(dir)
}

fn app() -> TestResult<Router> {
    let bundle = AssetBundle::load_dir(bundle_dir()?)?;
    Ok(router(Router::builder().assets(bundle)))
}

async fn send(
    method: Method,
    path: &str,
    cookie: Option<&str>,
) -> TestResult<Response> {
    let mut request = Request::builder().method(method.clone()).uri(path);
    if method == Method::POST {
        request = request.header(header::CONTENT_TYPE, FORM);
    }
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let body = if method == Method::POST {
        Body::from("guild=7&max_partners=3".to_owned())
    } else {
        Body::empty()
    };
    Ok(app()?.handle(request.body(body)?).await)
}

fn location(response: &Response) -> Option<&str> {
    response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok())
}

/// The `name=value` pair of the flash cookie a response sets.
fn flash_cookie(response: &Response) -> Option<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with(&format!("{FLASH_COOKIE}=")))
        .and_then(|value| value.split(';').next())
        .map(str::to_owned)
}

async fn body_text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

#[tokio::test]
async fn every_old_settings_address_moves_permanently_to_its_page() {
    for (slug, target) in MOVED {
        let path = format!("/guild/7/settings/{slug}");
        let response = send(Method::GET, &path, None).await.unwrap();

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT, "{path}");
        assert_eq!(location(&response), Some(target), "{path}");
        assert_eq!(flash_cookie(&response), None, "{path}");
    }
}

#[tokio::test]
async fn the_query_travels_with_the_move() {
    for (path, target) in [
        (
            "/guild/7/settings/patreon?patreon=connected",
            "/guild/7/patreon?patreon=connected",
        ),
        (
            "/guild/7/settings/youtube?youtube=declined",
            "/guild/7/youtube?youtube=declined",
        ),
        (
            "/guild/7/settings/levels?scope=global&page=2",
            "/guild/7/levels?scope=global&page=2",
        ),
        ("/guild/7/settings/temp-voice?created=1", "/guild/7/temp-voice?created=1"),
        (
            "/guild/7/settings/support?role-added=1&article-deleted=1",
            "/guild/7/support?role-added=1&article-deleted=1",
        ),
        ("/guild/7/settings/support?remove-role", "/guild/7/support?remove-role"),
        ("/guild/7/settings/bogus?x=%2F", "/guild/7/settings?x=%2F"),
    ] {
        let response = send(Method::GET, path, None).await.unwrap();

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT, "{path}");
        assert_eq!(location(&response), Some(target), "{path}");
    }
}

#[tokio::test]
async fn the_location_is_built_from_an_encoded_guild_segment() {
    let response =
        send(Method::GET, "/guild/a%20b%3F/settings/ai", None).await.unwrap();

    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(location(&response), Some("/guild/a%20b%3F/ai"));
}

#[tokio::test]
async fn a_trailing_slash_is_dropped_before_the_move() {
    let response = send(Method::GET, "/guild/7/settings/ai/", None).await.unwrap();

    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(location(&response), Some("/guild/7/settings/ai"));
}

#[tokio::test]
async fn a_form_posted_to_an_old_address_is_not_applied_and_says_so() {
    for (slug, target) in MOVED {
        for path in [
            format!("/guild/7/settings/{slug}"),
            format!("/guild/7/settings/{slug}?remove-role"),
        ] {
            let response = send(Method::POST, &path, None).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
            assert_eq!(location(&response), Some(target), "{path}");
            assert!(flash_cookie(&response).is_some(), "{path}");
        }
    }
}

#[tokio::test]
async fn the_not_saved_flash_shows_once_on_the_page_it_leads_to() {
    let response =
        send(Method::POST, "/guild/7/settings/music", None).await.unwrap();
    let cookie = flash_cookie(&response).unwrap();

    let page = send(Method::GET, "/guild/7/music", Some(&cookie)).await.unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    let cleared = page
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(|value| value.starts_with(&format!("{FLASH_COOKIE}=;")));
    assert!(cleared, "reading the flash clears it");
    let html = body_text(page).await.unwrap();
    assert!(
        html.contains(r#"<div class="flash-region" role="status"><p class="flash flash-error" data-flash="">"#),
        "{html}"
    );
    assert!(html.contains(NOT_SAVED), "{html}");
    assert_eq!(html.matches(NOT_SAVED).count(), 1, "{html}");

    let html = body_text(send(Method::GET, "/guild/7/music", None).await.unwrap())
        .await
        .unwrap();
    assert!(!html.contains(NOT_SAVED), "{html}");
}

#[tokio::test]
async fn an_unsigned_or_altered_flash_cookie_is_ignored() {
    let forged =
        Flash::new(FlashKind::Success, "Re-enter your token at evil.example");
    let response =
        send(Method::POST, "/guild/7/settings/music", None).await.unwrap();
    let signed = flash_cookie(&response).unwrap();
    let (name, value) = signed.split_once('=').unwrap();
    let swapped = if value.starts_with('A') { 'B' } else { 'A' };
    let altered = format!("{name}={swapped}{}", value.get(1..).unwrap());

    for cookie in [format!("{FLASH_COOKIE}={}", forged.encode()), altered] {
        let page = send(Method::GET, "/guild/7/music", Some(&cookie)).await.unwrap();
        let html = body_text(page).await.unwrap();

        assert!(!html.contains("evil.example"), "{cookie}: {html}");
        assert!(!html.contains(NOT_SAVED), "{cookie}: {html}");
        assert!(
            html.contains(r#"<div class="flash-region" role="status"></div>"#),
            "{cookie}: {html}"
        );
    }
}

#[tokio::test]
async fn the_new_addresses_answer_and_the_old_shapes_do_not_linger() {
    for path in [
        "/guild/7/settings",
        "/guild/7/ai",
        "/guild/7/support/suggestions",
        "/guild/7/support/faq",
        "/guild/7/support/faq/new",
        "/guild/7/support/faq/12",
        "/guild/7/support/wiki",
        "/guild/7/support/roles",
    ] {
        let response = send(Method::GET, path, None).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }

    let response =
        send(Method::GET, "/guild/7/settings/general/extra", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
