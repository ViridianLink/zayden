//! Every link the navigation builds from the module list resolves to a
//! registered page.

use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use http_body_util::BodyExt;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Method, Router, StatusCode};
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::nav::{self, Dest, MODULES};
use web::router;

const STYLESHEET_FILE: &str = "tailwind-0123456789abcdef.css";
const RUNTIME_FILE: &str = "topcoat-0123456789abcdef.js";
const PENDING_FILE: &str = "pending-submit-0123456789abcdef.js";

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// A bundle directory laid out like the asset bundler's output, written once
/// per test process since tests run concurrently.
fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-legacy-routes-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    for file in [STYLESHEET_FILE, RUNTIME_FILE, PENDING_FILE] {
        std::fs::write(dir.join(file), "")?;
    }
    std::fs::write(
        dir.join("manifest.toml"),
        format!(
            "version = 1\n\n\
             [[assets]]\nid = {}\nfile = \"{STYLESHEET_FILE}\"\nhash = \"0\"\ncontent_type = \"text/css\"\n\n\
             [[assets]]\nid = {}\nfile = \"{RUNTIME_FILE}\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n\n\
             [[assets]]\nid = {}\nfile = \"{PENDING_FILE}\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n",
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

async fn get(path: &str) -> TestResult<(StatusCode, String)> {
    let request =
        Request::builder().method(Method::GET).uri(path).body(Body::empty())?;
    let response: Response = app()?.handle(request).await;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok((status, String::from_utf8(bytes.to_vec())?))
}

#[tokio::test]
async fn every_module_link_resolves_to_a_page() {
    for module in MODULES {
        let href = module.href(1);
        let (status, html) = get(&href).await.unwrap();

        assert_ne!(status, StatusCode::NOT_FOUND, "{href}");
        if let Dest::Section { .. } = module.dest {
            assert_eq!(status, StatusCode::OK, "{href}");
            let title =
                format!("<title>{} - Zayden Dashboard</title>", module.label);
            assert!(html.contains(&title), "{href}");
        }
    }
}

#[tokio::test]
async fn the_settings_href_builds_the_route_each_section_is_served_on() {
    for module in MODULES {
        let Some(slug) = module.slug() else { continue };
        let href = nav::settings_href(7, slug).unwrap();

        assert_eq!(href, format!("/guild/7/{}", module.path()));
        assert_eq!(get(&href).await.unwrap().0, StatusCode::OK, "{href}");
    }
}

#[tokio::test]
async fn a_path_outside_the_route_table_is_not_found() {
    let (status, _) = get("/guild/1/settings/general/extra").await.unwrap();

    assert_eq!(status, StatusCode::NOT_FOUND);
}
