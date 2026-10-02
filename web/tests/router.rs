use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use http_body_util::BodyExt;
use topcoat::Result as ViewResult;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::error::not_found;
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Method, Router, StatusCode, header, page};
use topcoat::view::{View, view};
use web::document::{NOT_FOUND_TITLE, PAGE_TITLES, STYLESHEET};
use web::router;

const STYLESHEET_FILE: &str = "tailwind-0123456789abcdef.css";
const STYLESHEET_BODY: &str = ".login-page{display:grid}";
const RUNTIME_FILE: &str = "topcoat-0123456789abcdef.js";
const RUNTIME_BODY: &str = "export {};";

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[page("/test/missing-record")]
async fn missing_record() -> ViewResult<impl View> {
    Err::<(), _>(not_found())?;
    Ok(view! { <p>"found"</p> })
}

/// A bundle directory laid out like `topcoat asset bundle` output, holding
/// stand-ins for the stylesheet and the browser runtime. Written once per
/// test process, since tests run concurrently.
fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-router-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(STYLESHEET_FILE), STYLESHEET_BODY)?;
    std::fs::write(dir.join(RUNTIME_FILE), RUNTIME_BODY)?;
    std::fs::write(
        dir.join("manifest.toml"),
        format!(
            "version = 1\n\n\
             [[assets]]\nid = {}\nfile = \"{STYLESHEET_FILE}\"\nhash = \"0\"\ncontent_type = \"text/css\"\n\n\
             [[assets]]\nid = {}\nfile = \"{RUNTIME_FILE}\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n",
            STYLESHEET.id().as_u64(),
            topcoat::runtime::SCRIPT.id().as_u64(),
        ),
    )?;
    Ok(dir)
}

fn app() -> TestResult<Router> {
    let bundle = AssetBundle::load_dir(bundle_dir()?)?;
    Ok(router(Router::builder().assets(bundle).page(missing_record)))
}

async fn send(method: Method, path: &str) -> TestResult<Response> {
    let request = Request::builder().method(method).uri(path).body(Body::empty())?;
    Ok(app()?.handle(request).await)
}

async fn body_text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn sample_path(route: &str) -> String {
    route
        .split('/')
        .map(|segment| match segment {
            "{*rest}" => "missing",
            s if s.starts_with('{') => "1",
            s => s,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[tokio::test]
async fn unknown_path_is_404_with_not_found_card() {
    let response = send(Method::GET, "/does-not-exist").await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let html = body_text(response).await.unwrap();
    assert!(html.contains(
        "<body><div class=\"login-page\"><div class=\"hero-glow\"></div><div class=\"login-card\">"
    ));
    assert!(html.contains(
        "<span class=\"brand\"><span class=\"brand-mark\">Z</span>Zayden</span>"
    ));
    assert!(html.contains("<h1>404</h1>"));
    assert!(html.contains("<p>We couldn't find that page.</p>"));
    assert!(
        html.contains(
            "<a href=\"/\" class=\"btn btn-primary btn-lg\">Back home</a>"
        )
    );
    assert!(html.contains(
        "<nav class=\"legal-links\" aria-label=\"Legal\"><a href=\"/privacy\">Privacy Policy</a><a href=\"/terms\">Terms of Service</a></nav></div></body></html>"
    ));
}

#[tokio::test]
async fn nested_unknown_path_is_404() {
    let response = send(Method::GET, "/guild/1/nope/deeper").await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_path_is_404_for_every_method() {
    for method in [Method::POST, Method::PUT, Method::DELETE] {
        let response = send(method, "/does-not-exist").await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn document_head_renders_shell_stylesheet_and_runtime() {
    let html = body_text(send(Method::GET, "/does-not-exist").await.unwrap())
        .await
        .unwrap();
    assert!(html.starts_with(&format!(
        "<!DOCTYPE html><html lang=\"en\" data-bot=\"zayden\"><head>\
         <meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <link rel=\"stylesheet\" href=\"/_topcoat/assets/{STYLESHEET_FILE}\">\
         <script type=\"module\" src=\"/_topcoat/assets/{RUNTIME_FILE}\"></script>\
         <title>{NOT_FOUND_TITLE}</title></head><body>"
    )));
}

#[tokio::test]
async fn page_failing_with_not_found_renders_not_found_document() {
    let response = send(Method::GET, "/test/missing-record").await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let html = body_text(response).await.unwrap();
    assert_eq!(html.matches("<title>").count(), 1);
    assert!(html.contains(&format!("<title>{NOT_FOUND_TITLE}</title>")));
    assert!(html.contains("<h1>404</h1>"));
    assert!(!html.contains("found</p>"));
}

#[tokio::test]
async fn every_page_title_is_rendered() {
    for (route, title) in PAGE_TITLES {
        let response = send(Method::GET, &sample_path(route)).await.unwrap();
        if *route != "/{*rest}" {
            assert_ne!(response.status(), StatusCode::NOT_FOUND, "{route}");
        }
        let html = body_text(response).await.unwrap();
        assert!(html.contains(&format!("<title>{title}</title>")), "{route}");
    }
}

#[tokio::test]
async fn bundled_assets_are_served() {
    for (file, content_type, body) in [
        (STYLESHEET_FILE, "text/css", STYLESHEET_BODY),
        (RUNTIME_FILE, "text/javascript", RUNTIME_BODY),
    ] {
        let response =
            send(Method::GET, &format!("/_topcoat/assets/{file}")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{file}");
        assert_eq!(response.headers()[header::CONTENT_TYPE], content_type);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=31536000, immutable"
        );
        assert_eq!(body_text(response).await.unwrap(), body);
    }
}
