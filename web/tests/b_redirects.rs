use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, StatusCode, header};
use url::form_urlencoded;
use web::components::brand::LOGO;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::engagement::pages::{greeting_actions, reaction_role_actions};
use web::flash::FLASH_COOKIE;
use web::state::WebState;
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const INVITE: &str = "https://discord.com/oauth2/authorize?client_id=1&scope=bot";

/// A session cookie the cache does not hold, so its lookup reaches the
/// refusing database and the page shows its load error.
const UNCACHED: &str = "session=uncached-token";

const NOT_SAVED: &str =
    "This page was updated and your change was not saved. Please try again.";

/// Every success flag a page answered with before the flash replaced them.
const LEGACY_FLAGS: &str = "added=1&removed=1&created=1&deleted=1&role-added=1&role-removed=1\
     &link-added=1&link-removed=1&article-created=1&article-deleted=1&channel_added=1\
     &channel_removed=1&image_added=1&image_removed=1&disconnected=1&remove-role=1&saved=1\
     &linked=1&toggled=1";

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-b-redirects-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let files = [
        ("tailwind-0123456789abcdef.css", "text/css", STYLESHEET.id().as_u64()),
        (
            "topcoat-0123456789abcdef.js",
            "text/javascript",
            topcoat::runtime::SCRIPT.id().as_u64(),
        ),
        (
            "pending-submit-0123456789abcdef.js",
            "text/javascript",
            PENDING_SUBMIT.id().as_u64(),
        ),
        ("logo-0123456789abcdef.png", "image/png", LOGO.id().as_u64()),
    ];
    let mut manifest = "version = 1\n".to_owned();
    for (file, content_type, id) in files {
        std::fs::write(dir.join(file), "")?;
        write!(
            manifest,
            "\n[[assets]]\nid = {id}\nfile = \"{file}\"\nhash = \"0\"\ncontent_type = \"{content_type}\"\n"
        )?;
    }
    std::fs::write(dir.join("manifest.toml"), manifest)?;
    Ok(dir)
}

fn config() -> BotConfig {
    BotConfig {
        discord_token: "test-bot-token".to_owned(),
        bungie_api_key: String::new(),
        ai_provider_key: String::new(),
        google_api_key: String::new(),
        discord_client_secret: "test-client-secret".to_owned(),
        spotify: None,
        ai_api_endpoint: String::new(),
        ai_model: String::new(),
        ai_model_structured: String::new(),
        ai_model_pro: String::new(),
        bot_owner: 0,
        zayden_guild: 0,
        llamad2_guild: 0,
        zayden_id: 1,
        error_log_webhook: None,
        normal_log_webhook: None,
        flaresolverr_url: None,
        youtube_cookies: None,
        palworld_paldex_url: None,
        palworld_palcalc_url: None,
        palworld_save_dir: None,
        palworld_uploads_dir: PathBuf::new(),
        pelican: None,
        jellyfin: None,
        hosting: None,
        redirect_uri: "http://localhost:3000/auth/callback".to_owned(),
        bind_addr: "127.0.0.1:0".to_owned(),
        invite_url: Some(INVITE.to_owned()),
        upgrade_url: None,
        kofi_verification_token: None,
        patreon: None,
        youtube: None,
        discord_sku_pro: None,
        discord_sku_ultra: None,
        radio_stations: Arc::from(Vec::new()),
    }
}

/// The router over a database that refuses connections: signed-out visitors
/// need no query.
fn router() -> TestResult<Router> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let config = config();
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy("postgres://127.0.0.1:1/unused")?;
    let app = Arc::new(ZaydenAppState::new(pool, &config));
    let state = WebState::new(app, &config)?;
    let base = Router::builder()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .app_context(state);
    Ok(web::router(base))
}

struct Answer {
    status: StatusCode,
    location: Option<String>,
    flash: Option<String>,
    html: String,
}

async fn answer(response: Response) -> TestResult<Answer> {
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let set = format!("{FLASH_COOKIE}=");
    let cleared = format!("{FLASH_COOKIE}=;");
    let flash = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with(&set) && !value.starts_with(&cleared))
        .and_then(|value| value.split(';').next())
        .map(str::to_owned);
    let bytes = response.into_body().collect().await?.to_bytes();
    let html = String::from_utf8(bytes.to_vec())?;
    Ok(Answer { status, location, flash, html })
}

async fn get(
    router: &Router,
    path: &str,
    cookie: Option<&str>,
) -> TestResult<Answer> {
    let mut request = Request::get(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    answer(router.handle(request.body(Body::empty())?).await).await
}

async fn post(
    router: &Router,
    path: &str,
    fields: &[(&str, &str)],
    cookie: Option<&str>,
) -> TestResult<Answer> {
    let body = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(fields)
        .finish();
    let mut request = Request::post(path)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    answer(router.handle(request.body(Body::from(body))?).await).await
}

fn flash_count(html: &str) -> usize {
    html.matches(r#"<span class="flash-text">"#).count()
}

#[tokio::test]
async fn kept_public_addresses_still_answer_where_they_did() {
    let router = router().unwrap();

    for path in
        ["/", "/login", "/login?error=auth_failed", "/upgrade", "/privacy", "/terms"]
    {
        let reply = get(&router, path, None).await.unwrap();
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        assert_eq!(reply.location, None, "{path}");
    }

    let invite = get(&router, "/invite", None).await.unwrap();
    assert!(invite.status.is_redirection(), "{}", invite.status);
    assert_eq!(invite.location.as_deref(), Some(INVITE));
    let preselected = get(&router, "/invite?guild=7", None).await.unwrap();
    assert_eq!(preselected.status, invite.status);
    assert_eq!(
        preselected.location.as_deref(),
        Some(format!("{INVITE}&guild_id=7&disable_guild_select=true").as_str())
    );

    let upgrade =
        post(&router, "/upgrade", &[("email", "someone@example.com")], None)
            .await
            .unwrap();
    assert_eq!(upgrade.status, StatusCode::SEE_OTHER);
    assert_eq!(upgrade.location.as_deref(), Some("/login"));
}

#[tokio::test]
async fn kept_dashboard_and_admin_addresses_send_visitors_to_sign_in() {
    let router = router().unwrap();

    for path in [
        "/guilds",
        "/guild/7",
        "/guild/7/levels",
        "/guild/7/levels?page=2&scope=global",
        "/guild/7/reaction-roles",
        "/guild/7/greetings",
        "/admin/servers",
        "/admin/destiny2/loadouts",
        "/admin/destiny2/loadouts/new",
        "/admin/destiny2/loadouts/5",
    ] {
        let reply = get(&router, path, None).await.unwrap();
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{path}");
        assert_eq!(reply.location.as_deref(), Some("/login"), "{path}");
        assert_eq!(reply.flash, None, "{path}");
    }

    let toggle =
        post(&router, "/guild/7", &[("guild", "7"), ("module", "gambling")], None)
            .await
            .unwrap();
    assert_eq!(toggle.status, StatusCode::SEE_OTHER);
    assert_eq!(toggle.location.as_deref(), Some("/login"));

    for path in [
        "/guild/7/greetings",
        "/guild/7/greetings?action=save-messages",
        "/guild/7/reaction-roles?action=add",
        "/guild/7/greetings/save-messages",
        "/guild/7/reaction-roles/add",
    ] {
        let reply = post(&router, path, &[("guild", "7")], None).await.unwrap();
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{path}");
        assert_eq!(reply.location.as_deref(), Some("/login"), "{path}");
        assert_eq!(reply.flash, None, "{path}");
    }
}

#[tokio::test]
async fn a_post_to_an_engagement_page_address_saves_nothing_and_says_so() {
    let router = router().unwrap();

    for page in ["greetings", "reaction-roles"] {
        let target = format!("/guild/7/{page}");
        let names = if page == "greetings" {
            greeting_actions()
        } else {
            reaction_role_actions()
        };
        let mut paths = vec![target.clone(), format!("{target}?action=bogus")];
        paths.extend(names.iter().map(|name| format!("{target}?action={name}")));

        for path in paths {
            let reply = post(&router, &path, &[("guild", "7")], Some(UNCACHED))
                .await
                .unwrap();
            assert_eq!(reply.status, StatusCode::SEE_OTHER, "{path}");
            assert_eq!(reply.location.as_deref(), Some(target.as_str()), "{path}");
            let flash = reply.flash.unwrap_or_else(|| panic!("{path}: no flash"));

            let landed =
                get(&router, &target, Some(&format!("{UNCACHED}; {flash}")))
                    .await
                    .unwrap();
            assert_eq!(landed.status, StatusCode::OK, "{path}");
            assert_eq!(flash_count(&landed.html), 1, "{path}: {}", landed.html);
            assert!(landed.html.contains(NOT_SAVED), "{path}: {}", landed.html);
        }
    }
}

#[tokio::test]
async fn a_legacy_post_keeps_the_guild_segment_encoded_in_its_location() {
    let router = router().unwrap();

    for (path, location) in [
        ("/guild/a%20b/greetings", "/guild/a%20b/greetings"),
        ("/guild/7%3F/reaction-roles?action=add", "/guild/7%3F/reaction-roles"),
        (
            "/guild/%E2%9C%93/greetings?action=save-messages",
            "/guild/%E2%9C%93/greetings",
        ),
    ] {
        let reply = post(&router, path, &[], Some(UNCACHED)).await.unwrap();
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{path}");
        assert_eq!(reply.location.as_deref(), Some(location), "{path}");
        assert!(reply.flash.is_some(), "{path}");
    }
}

#[tokio::test]
async fn every_legacy_action_name_has_its_own_address() {
    let router = router().unwrap();

    let greetings = greeting_actions();
    for name in [
        "save-messages",
        "save-cooldowns",
        "add-channel",
        "remove-channel",
        "add-image",
        "remove-image",
        "module",
    ] {
        assert!(greetings.contains(&name), "{name}: {greetings:?}");
    }
    let reaction_roles = reaction_role_actions();
    for name in ["add", "remove"] {
        assert!(reaction_roles.contains(&name), "{name}: {reaction_roles:?}");
    }

    for (page, names) in
        [("greetings", &greetings), ("reaction-roles", &reaction_roles)]
    {
        for name in names {
            let path = format!("/guild/7/{page}/{name}");
            let reply = post(&router, &path, &[("guild", "7")], Some(UNCACHED))
                .await
                .unwrap();
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
            assert_eq!(reply.location, None, "{path}");
            assert_eq!(reply.flash, None, "{path}");
            assert!(
                reply.html.contains(r#"role="alert""#),
                "{path}: {}",
                reply.html
            );
        }
        for unknown in ["bogus", "save", "levels"] {
            let path = format!("/guild/7/{page}/{unknown}");
            let reply = post(&router, &path, &[("guild", "7")], Some(UNCACHED))
                .await
                .unwrap();
            assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        }
    }
}

#[tokio::test]
async fn legacy_flags_are_ignored_on_every_page() {
    let router = router().unwrap();

    for page in [
        "/guild/7",
        "/guild/7/levels",
        "/guild/7/reaction-roles",
        "/guild/7/greetings",
    ] {
        let path = format!("{page}?{LEGACY_FLAGS}");
        let reply = get(&router, &path, Some(UNCACHED)).await.unwrap();
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        assert_eq!(reply.location, None, "{path}");
        assert_eq!(flash_count(&reply.html), 0, "{path}: {}", reply.html);
        assert_eq!(reply.flash, None, "{path}");
    }

    let upgrade =
        get(&router, &format!("/upgrade?{LEGACY_FLAGS}"), None).await.unwrap();
    assert_eq!(upgrade.status, StatusCode::OK);
    assert_eq!(flash_count(&upgrade.html), 0, "{}", upgrade.html);
}
