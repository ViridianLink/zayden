use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use topcoat::Result as ViewResult;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, RouterBuilder, StatusCode, header, page};
use topcoat::runtime::RouterBuilderRuntimeExt;
use topcoat::view::{View, view};
use web::auth::SessionUser;
use web::components::brand::LOGO;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::public::layout::{SessionSlot, public_nav};
use web::state::{SessionIdentity, SessionUsersCache, WebState};
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-public-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("tailwind-0123456789abcdef.css"), "")?;
    std::fs::write(dir.join("topcoat-0123456789abcdef.js"), "")?;
    std::fs::write(dir.join("pending-submit-0123456789abcdef.js"), "")?;
    std::fs::write(dir.join("logo-0123456789abcdef.png"), "")?;
    std::fs::write(
        dir.join("manifest.toml"),
        format!(
            "version = 1\n\n\
             [[assets]]\nid = {}\nfile = \"tailwind-0123456789abcdef.css\"\nhash = \"0\"\ncontent_type = \"text/css\"\n\n\
             [[assets]]\nid = {}\nfile = \"topcoat-0123456789abcdef.js\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n\n\
             [[assets]]\nid = {}\nfile = \"pending-submit-0123456789abcdef.js\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n\n\
             [[assets]]\nid = {}\nfile = \"logo-0123456789abcdef.png\"\nhash = \"0\"\ncontent_type = \"image/png\"\n",
            STYLESHEET.id().as_u64(),
            topcoat::runtime::SCRIPT.id().as_u64(),
            PENDING_SUBMIT.id().as_u64(),
            LOGO.id().as_u64(),
        ),
    )?;
    Ok(dir)
}

fn config(invite_url: Option<&str>) -> BotConfig {
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
        invite_url: invite_url.map(str::to_owned),
        upgrade_url: None,
        kofi_verification_token: None,
        patreon: None,
        youtube: None,
        discord_sku_pro: None,
        discord_sku_ultra: None,
        radio_stations: Arc::from(Vec::new()),
    }
}

/// App state over a pool whose database refuses connections: signed-out
/// visitors need no query, cached sessions skip it, and any other cookie
/// makes the session lookup fail.
async fn web_state(invite_url: Option<&str>) -> TestResult<WebState> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let config = config(invite_url);
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy("postgres://127.0.0.1:1/unused")?;
    let app = Arc::new(ZaydenAppState::new(pool, &config));
    let state = WebState::new(app, &config)?;
    state
        .sessions
        .insert("live-token".to_owned(), SessionIdentity {
            user_id: 41,
            access_token: "access".to_owned(),
        })
        .await;
    seed_users(&state.discord.users, &[41]).await;
    Ok(state)
}

fn builder() -> TestResult<RouterBuilder> {
    Ok(Router::builder().assets(AssetBundle::load_dir(bundle_dir()?)?))
}

fn app(state: Option<WebState>) -> TestResult<Router> {
    let base = builder()?;
    let base = match state {
        Some(state) => base.app_context(state),
        None => base,
    };
    Ok(web::router(base))
}

async fn get(
    router: &Router,
    path: &str,
    cookie: Option<&str>,
) -> TestResult<Response> {
    let mut request = Request::get(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    Ok(router.handle(request.body(Body::empty())?).await)
}

async fn body_text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

const INVITE: &str = "https://discord.com/oauth2/authorize?client_id=1&permissions=8&integration_type=0&scope=bot+applications.commands";

async fn page_html(path: &str) -> TestResult<String> {
    let router = app(Some(web_state(Some(INVITE)).await?))?;
    let response = get(&router, path, None).await?;
    if response.status() != StatusCode::OK {
        return Err(format!("{path} answered {}", response.status()).into());
    }
    body_text(response).await
}

const LEGAL_MAIN: &str =
    r#"<main id="main" class="public-main" tabindex="-1"><div class="legal">"#;

fn between<'a>(html: &'a str, start: &str, end: &str) -> TestResult<&'a str> {
    let (_, rest) =
        html.split_once(start).ok_or_else(|| format!("missing {start}"))?;
    let (inner, _) = rest.split_once(end).ok_or_else(|| format!("missing {end}"))?;
    Ok(inner)
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {},
        }
    }
    out.replace("&amp;", "&").replace("&#39;", "'").replace("&quot;", "\"")
}

/// Inner HTML of every `<tag ...>...</tag>`.
fn elements<'a>(html: &'a str, tag: &str) -> Vec<&'a str> {
    let close = format!("</{tag}>");
    let mut found = Vec::new();
    let mut rest = html;
    while let Some((_, after)) = rest.split_once(&format!("<{tag}")) {
        let Some(first) = after.chars().next() else { break };
        if first != '>' && first != ' ' {
            rest = after;
            continue;
        }
        let Some((_, body)) = after.split_once('>') else { break };
        let Some((inner, tail)) = body.split_once(&close) else { break };
        found.push(inner);
        rest = tail;
    }
    found
}

fn texts(html: &str, tag: &str) -> Vec<String> {
    elements(html, tag).into_iter().map(strip_tags).collect()
}

fn hrefs(html: &str) -> Vec<String> {
    html.split("<a ")
        .skip(1)
        .filter_map(|anchor| anchor.split_once("href=\"")?.1.split_once('"'))
        .map(|(href, _)| href.to_owned())
        .collect()
}

fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

#[page("/nav/signed-out")]
async fn nav_signed_out() -> ViewResult<impl View> {
    Ok(view! { public_nav(session: SessionSlot::SignedOut) })
}

#[page("/nav/unavailable")]
async fn nav_unavailable() -> ViewResult<impl View> {
    Ok(view! { public_nav(session: SessionSlot::Unavailable) })
}

#[page("/nav/signed-in")]
async fn nav_signed_in() -> ViewResult<impl View> {
    Ok(view! { public_nav(session: SessionSlot::SignedIn) })
}

async fn nav_html(builder: RouterBuilder, path: &str) -> TestResult<String> {
    let router = builder.runtime().build();
    let response = router.handle(Request::get(path).body(Body::empty())?).await;
    body_text(response).await
}

#[tokio::test]
async fn landing_has_the_public_chrome_and_hero() {
    let html = page_html("/").await.unwrap();
    assert!(html.contains("<title>Zayden — the all-in-one Discord bot</title>"));

    assert!(html.starts_with(
        "<!DOCTYPE html><html lang=\"en\" class=\"dark\" data-bot=\"zayden\">"
    ));
    assert!(html.contains(r#"<div class="public"><header class="public-header">"#));
    assert!(html.contains(
        r#"<a href="/" class="brand"><img class="brand-mark" src="/_topcoat/assets/logo-0123456789abcdef.png" alt="" width="28" height="28"><span class="brand-name">Zayden</span></a>"#
    ));
    assert_eq!(
        vec!["The Discord bot that grows with your server"],
        texts(&html, "h1"),
    );
    assert!(html.contains(r#"<h1 class="hero-title">The Discord bot that <span class="accent-text">grows with your server</span></h1>"#));
    assert!(html.contains(r#"<p class="hero-subtitle">Music, economy, moderation tools and more — configured from a clean dashboard, enforced natively by Discord. Free to run, Pro only where it costs us.</p>"#));
    assert!(html.contains("One bot. Every module.</span>"));
    assert_eq!(
        1,
        count(
            &html,
            r#"<main id="main" class="public-main" tabindex="-1"><div class="landing">"#
        )
    );
    assert_eq!(1, count(&html, "<footer class=\"footer\">"));
}

#[tokio::test]
async fn landing_links_keep_their_order_and_external_rel() {
    let html = page_html("/").await.unwrap();

    assert_eq!(
        [
            "#main",
            "/",
            "/#features",
            "/upgrade",
            "/auth/discord",
            "/#features",
            "/upgrade",
            "/auth/discord",
            "/invite",
            "/auth/discord",
            "/invite",
            "/upgrade",
            "/invite",
            "/upgrade",
            "/auth/discord",
            "/privacy",
            "/terms",
        ],
        hrefs(&html).as_slice(),
    );
    assert_eq!(
        2,
        count(
            &html,
            r#"<a href="/#features" rel="external" class="public-nav-link">Features</a>"#
        )
    );
    assert_eq!(
        2,
        count(&html, r#"<a href="/upgrade" class="public-nav-link">Pricing</a>"#)
    );
    assert_eq!(
        2,
        count(
            &html,
            r#"<a href="/auth/discord" rel="external" class="btn btn-secondary">Sign in</a>"#
        )
    );
    assert!(html.contains(r#"<a href="/auth/discord" rel="external" class="btn btn-secondary btn-lg">Sign in with Discord<svg"#));
    assert!(!html.contains("Open dashboard"));
    assert!(html.contains(
        r#"<a href="/upgrade" class="btn btn-ghost btn-lg">See plans</a>"#
    ));
}

#[tokio::test]
async fn landing_sends_a_signed_in_visitor_to_their_servers() {
    let router = app(Some(web_state(Some(INVITE)).await.unwrap())).unwrap();
    let response = get(&router, "/", Some("session=live-token")).await.unwrap();
    assert_eq!(StatusCode::OK, response.status());
    let html = body_text(response).await.unwrap();

    assert!(html.contains(r#"<div class="hero-actions"><a href="/invite" rel="external" class="btn btn-primary btn-lg">"#));
    assert!(html.contains(
        r#"<a href="/guilds" class="btn btn-secondary btn-lg">Open dashboard<svg"#
    ));
    assert!(!html.contains("Sign in with Discord"));
    assert_eq!(
        2,
        count(
            &html,
            r#"<a href="/guilds" class="btn btn-primary">Open dashboard</a>"#
        )
    );
}

#[tokio::test]
async fn landing_feature_grid_lists_six_tinted_modules() {
    let html = page_html("/").await.unwrap();
    let grid = between(&html, "<div class=\"feature-grid\">", "</section>").unwrap();

    assert_eq!(
        vec![
            "Music",
            "Economy & Games",
            "Family",
            "Palworld",
            "Tickets & Support",
            "Marathon",
        ],
        texts(grid, "h3"),
    );
    let tints: Vec<&str> = grid
        .split("style=\"--tint: ")
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(tint, _)| tint))
        .collect();
    assert_eq!(
        ["#a78bfa", "#f472b6", "#fb7185", "#34d399", "#38bdf8", "#f59e0b"],
        tints.as_slice(),
    );
    assert_eq!(6, count(grid, "<div class=\"feature-card\">"));
    assert!(grid.contains(
        "<p>Currency, a shop, leaderboards, and a dozen mini-games to play.</p>"
    ));
    assert!(grid.contains(
        "<p>Wiki lookups and news for the games your members care about.</p>"
    ));
}

#[tokio::test]
async fn landing_sections_and_call_to_action_copy() {
    let html = page_html("/").await.unwrap();

    assert_eq!(
        vec!["Everything your community needs", "Ready in under a minute"],
        texts(&html, "h2"),
    );
    assert!(html.contains(r#"<section id="features" class="landing-section landing-block"><div class="section-head">"#));
    assert!(html.contains("<p>Toggle modules on or off per server. Each one plugs straight into Discord's command-permission system.</p>"));
    assert!(html.contains(r#"<section class="cta-band"><h2>Ready in under a minute</h2><p>Invite Zayden, sign in with Discord, and start configuring your server from the dashboard.</p>"#));
}

#[tokio::test]
async fn footer_lists_the_five_links() {
    let html = page_html("/privacy").await.unwrap();
    let footer = between(&html, "<footer class=\"footer\">", "</footer>").unwrap();

    assert_eq!(
        concat!(
            r#"<div class="footer-inner"><span>© 2026 Zayden. Not affiliated with Discord.</span>"#,
            r#"<nav class="footer-links" aria-label="Footer">"#,
            r#"<a href="/invite" rel="external">Invite</a>"#,
            r#"<a href="/upgrade">Pricing</a>"#,
            r#"<a href="/auth/discord" rel="external">Sign in</a>"#,
            r#"<a href="/privacy">Privacy Policy</a>"#,
            r#"<a href="/terms">Terms of Service</a>"#,
            "</nav></div>",
        ),
        footer,
    );
}

#[tokio::test]
async fn header_slot_is_empty_when_the_session_lookup_fails() {
    let html =
        nav_html(builder().unwrap().page(nav_unavailable), "/nav/unavailable")
            .await
            .unwrap();

    assert!(!html.contains("Sign in"));
    assert!(!html.contains("Open dashboard"));
    assert!(
        html.contains(r#"<a href="/upgrade" class="public-nav-link">Pricing</a>"#)
    );
}

#[tokio::test]
async fn header_slot_offers_login_when_signed_out() {
    let html = nav_html(builder().unwrap().page(nav_signed_out), "/nav/signed-out")
        .await
        .unwrap();

    assert!(html.contains(
        r#"<a href="/upgrade" class="public-nav-link">Pricing</a><a href="/auth/discord" rel="external" class="btn btn-secondary">Sign in</a>"#
    ));
    assert!(!html.contains("Open dashboard"));
}

#[tokio::test]
async fn header_slot_offers_the_dashboard_when_signed_in() {
    let html = nav_html(builder().unwrap().page(nav_signed_in), "/nav/signed-in")
        .await
        .unwrap();

    assert!(html.contains(
        r#"<a href="/guilds" class="btn btn-primary">Open dashboard</a>"#
    ));
    assert!(!html.contains(">Sign in<"));
}

#[test]
fn session_slot_maps_lookup_outcomes() {
    use web::auth::AuthError;

    assert_eq!(SessionSlot::SignedOut, SessionSlot::from(Ok(false)));
    assert_eq!(SessionSlot::SignedIn, SessionSlot::from(Ok(true)));
    assert_eq!(
        SessionSlot::Unavailable,
        SessionSlot::from(Err(AuthError::Unauthenticated)),
    );
}

#[tokio::test]
async fn login_renders_the_card_in_a_main_landmark() {
    let html = page_html("/login").await.unwrap();
    assert!(html.contains("<title>Sign In - Zayden Dashboard</title>"));

    assert!(html.contains(concat!(
        r##"<body><a class="skip-link" href="#main">Skip to main content</a><main class="login-page" id="main" tabindex="-1"><div class="hero-glow"></div><div class="login-card">"##,
        r#"<span class="brand"><img class="brand-mark" src="/_topcoat/assets/logo-0123456789abcdef.png" alt="" width="28" height="28">Zayden</span>"#,
        "<h1>Welcome back</h1>",
        "<p>Connect your Discord account to manage your server settings.</p>",
        r#"<a href="/auth/discord" rel="external" class="btn btn-primary btn-lg">Sign in with Discord</a></div>"#,
        r#"<nav class="legal-links" aria-label="Legal"><a href="/privacy">Privacy Policy</a><a href="/terms">Terms of Service</a></nav></main></body>"#,
    )));
    assert_eq!(1, count(&html, "<main"));
    assert!(!html.contains("<header"));
    assert!(!html.contains(r#"role="alert""#));
}

#[tokio::test]
async fn login_explains_a_failed_sign_in_and_offers_a_retry() {
    let plain = page_html("/login").await.unwrap();
    let failed = page_html("/login?error=auth_failed").await.unwrap();

    assert!(failed.contains(concat!(
        "<p>Connect your Discord account to manage your server settings.</p>",
        r#"<p class="error" role="alert">Signing in with Discord didn't finish, so you aren't signed in. Please try again.</p>"#,
        r#"<a href="/auth/discord" rel="external" class="btn btn-primary btn-lg">Try again with Discord</a></div>"#,
    )));
    assert_eq!(plain, page_html("/login?error=other").await.unwrap());
    assert_eq!(plain, page_html("/login?unrelated=auth_failed").await.unwrap());
}

#[tokio::test]
async fn login_sends_a_signed_in_visitor_to_their_servers() {
    let router = app(Some(web_state(None).await.unwrap())).unwrap();
    let response = get(&router, "/login", Some("session=live-token")).await.unwrap();

    assert_eq!(StatusCode::SEE_OTHER, response.status());
    assert_eq!("/guilds", response.headers()[header::LOCATION]);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
    assert_eq!("", body_text(response).await.unwrap());
}

#[tokio::test]
async fn login_still_offers_sign_in_in_a_main_landmark_when_the_session_lookup_fails()
 {
    let router = app(Some(web_state(None).await.unwrap())).unwrap();
    let response =
        get(&router, "/login", Some("session=unknown-token")).await.unwrap();

    assert_eq!(StatusCode::OK, response.status());
    let html = body_text(response).await.unwrap();
    assert!(html.contains(
        r##"<body><a class="skip-link" href="#main">Skip to main content</a><main class="login-page" id="main" tabindex="-1">"##
    ));
    assert!(html.contains(
        r#"<a href="/auth/discord" rel="external" class="btn btn-primary btn-lg">Sign in with Discord</a>"#
    ));
}

#[tokio::test]
async fn pages_keep_rendering_with_an_empty_header_slot_when_the_session_lookup_fails()
 {
    let router = app(Some(web_state(None).await.unwrap())).unwrap();
    let response =
        get(&router, "/privacy", Some("session=unknown-token")).await.unwrap();

    assert_eq!(StatusCode::OK, response.status());
    let html = body_text(response).await.unwrap();
    assert!(!html.contains(r#"class="btn btn-secondary">Sign in</a>"#));
    assert!(!html.contains("Open dashboard"));
    assert!(
        html.contains(r#"<a href="/upgrade" class="public-nav-link">Pricing</a>"#)
    );
    assert!(html.contains(LEGAL_MAIN));
}

#[tokio::test]
async fn invite_redirects_to_the_configured_url() {
    let router = app(Some(web_state(Some(INVITE)).await.unwrap())).unwrap();
    let response = get(&router, "/invite", None).await.unwrap();

    assert_eq!(StatusCode::SEE_OTHER, response.status());
    assert_eq!(INVITE, response.headers()[header::LOCATION]);
}

#[tokio::test]
async fn invite_preselects_the_server_on_a_discord_authorize_address() {
    let router = app(Some(web_state(Some(INVITE)).await.unwrap())).unwrap();

    let response =
        get(&router, "/invite?guild=1222360995700150443", None).await.unwrap();
    assert_eq!(StatusCode::SEE_OTHER, response.status());
    assert_eq!(
        format!("{INVITE}&guild_id=1222360995700150443&disable_guild_select=true"),
        response.headers()[header::LOCATION]
    );

    for ignored in [
        "/invite?guild=abc",
        "/invite?guild=0",
        "/invite?guild=",
        "/invite?guild=-7",
        "/invite?other=7",
    ] {
        let response = get(&router, ignored, None).await.unwrap();
        assert_eq!(StatusCode::SEE_OTHER, response.status(), "{ignored}");
        assert_eq!(INVITE, response.headers()[header::LOCATION], "{ignored}");
    }
}

#[tokio::test]
async fn invite_replaces_a_preselected_server_and_leaves_other_addresses_alone() {
    let preselected =
        "https://discord.com/api/oauth2/authorize?client_id=1&guild_id=9&scope=bot";
    let router = app(Some(web_state(Some(preselected)).await.unwrap())).unwrap();
    let response = get(&router, "/invite?guild=7", None).await.unwrap();
    assert_eq!(
        "https://discord.com/api/oauth2/authorize?client_id=1&scope=bot&guild_id=7&disable_guild_select=true",
        response.headers()[header::LOCATION]
    );

    for other in [
        "https://example.com/oauth2/authorize?client_id=1",
        "http://discord.com/oauth2/authorize?client_id=1",
        "https://discord.com/invite/zayden",
    ] {
        let router = app(Some(web_state(Some(other)).await.unwrap())).unwrap();
        let response = get(&router, "/invite?guild=7", None).await.unwrap();
        assert_eq!(StatusCode::SEE_OTHER, response.status(), "{other}");
        assert_eq!(other, response.headers()[header::LOCATION], "{other}");
    }
}

#[tokio::test]
async fn invite_is_an_empty_404_without_a_configured_url() {
    let router = app(Some(web_state(None).await.unwrap())).unwrap();
    let response = get(&router, "/invite", None).await.unwrap();

    assert_eq!(StatusCode::NOT_FOUND, response.status());
    assert!(response.headers().get(header::LOCATION).is_none());
    assert_eq!("", body_text(response).await.unwrap());
}

#[tokio::test]
async fn privacy_lists_its_sections_in_order() {
    let html = page_html("/privacy").await.unwrap();
    assert!(html.contains("<title>Privacy Policy - Zayden</title>"));
    let main = between(&html, LEGAL_MAIN, "</main>").unwrap();

    assert_eq!(vec!["Privacy Policy"], texts(main, "h1"));
    assert!(main.contains(
        r#"<h1>Privacy Policy</h1><p class="legal-updated">Last updated: 26 September 2026</p><p class="legal-lead">"#
    ));
    assert_eq!(
        [
            "discord",
            "features",
            "ai",
            "payments",
            "patreon",
            "youtube",
            "watch",
            "palworld",
            "third-parties",
            "security",
            "retention",
            "rights",
            "children",
            "changes",
        ]
        .map(|id| format!("<section id=\"{id}\">")),
        main.split("<section id=\"")
            .skip(1)
            .map(|rest| format!(
                "<section id=\"{}\">",
                rest.split('"').next().unwrap_or("")
            ))
            .collect::<Vec<_>>()
            .as_slice(),
    );
    assert_eq!(
        vec![
            "Information from Discord",
            "Data stored by each feature",
            "AI features",
            "Payments and entitlements",
            "Patreon connection",
            "YouTube connection",
            "Jellyfin and Watch",
            "Palworld",
            "Third parties",
            "Storage and security",
            "How long data is kept",
            "Your choices and rights",
            "Children",
            "Changes to this policy",
        ],
        texts(main, "h2"),
    );
    assert_eq!(
        vec![
            "Identifiers",
            "Dashboard sign-in",
            "Server settings",
            "Ko-fi",
            "Discord purchases",
            "Game-server hosting",
            "When you connect",
            "After you connect",
            "What is stored",
            "Disconnecting and revoking",
        ],
        texts(main, "h3"),
    );
}

#[tokio::test]
async fn privacy_markup_counts() {
    let html = page_html("/privacy").await.unwrap();
    let main = between(&html, LEGAL_MAIN, "</main>").unwrap();

    assert_eq!(40, count(main, "<strong>"));
    assert_eq!(3, count(main, "<strong>kilooscarsix@gmail.com</strong>"));
    assert_eq!(1, count(main, "<code>"));
    assert_eq!(12, count(main, "<ul>"));
    assert_eq!(74, count(main, "<li>"));
    assert_eq!(2, count(main, "class=\"legal-callout\""));
    assert_eq!(1, count(main, "class=\"legal-lead\""));
    assert!(!main.contains("mailto:"));
}

#[tokio::test]
async fn privacy_links_are_plain_outbound_anchors() {
    let html = page_html("/privacy").await.unwrap();
    let main = between(&html, LEGAL_MAIN, "</main>").unwrap();

    assert_eq!(
        [
            "#discord",
            "#features",
            "#ai",
            "#payments",
            "#patreon",
            "#youtube",
            "#watch",
            "#palworld",
            "#third-parties",
            "#security",
            "#retention",
            "#rights",
            "#children",
            "#changes",
            "https://security.google.com/settings/security/permissions",
            "https://policies.google.com/privacy",
            "https://developers.google.com/terms/api-services-user-data-policy",
            "https://discord.com/privacy",
        ],
        hrefs(main).as_slice(),
    );
    assert!(!main.contains("target="));
    assert!(main.contains(
        r#"<a href="https://security.google.com/settings/security/permissions">https://security.google.com/settings/security/permissions</a>"#
    ));
}

#[tokio::test]
async fn privacy_keeps_the_no_tracking_statement_and_retention_window() {
    let html = page_html("/privacy").await.unwrap();
    let text = strip_tags(between(&html, LEGAL_MAIN, "</main>").unwrap());

    assert!(text.contains("We do not sell personal information, and we do not use it for advertising. The dashboard sets no analytics or advertising cookies."));
    assert!(text.contains("kept for 30 days"));
    assert!(
        text.contains(
            "Questions or requests about your data: kilooscarsix@gmail.com."
        )
    );
}

#[tokio::test]
async fn terms_lists_its_sections_in_order() {
    let html = page_html("/terms").await.unwrap();
    assert!(html.contains("<title>Terms of Service - Zayden</title>"));
    let main = between(&html, LEGAL_MAIN, "</main>").unwrap();

    assert!(main.contains(
        r##"<h1>Terms of Service</h1><p class="legal-updated">Last updated: 26 September 2026</p><nav class="legal-callout" aria-labelledby="legal-contents"><p class="label" id="legal-contents">On this page</p><ul><li><a href="#acceptance">Acceptance</a></li><li><a href="#discord">Discord's rules</a></li>"##
    ));
    assert!(main.contains(
        r##"<li><a href="#governing-law">Governing law</a></li></ul></nav><section id="acceptance"><h2>Acceptance</h2><p class="legal-lead">"##
    ));
    assert_eq!(
        vec![
            "Acceptance",
            "Discord's rules",
            "Acceptable use",
            "Server admins",
            "Paid features",
            "No warranty and limited liability",
            "Termination",
            "Changes and contact",
            "Governing law",
        ],
        texts(main, "h2"),
    );
    assert_eq!(vec!["Game-server hosting"], texts(main, "h3"));
    let ids: Vec<&str> = main
        .split("<section id=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect();
    assert_eq!(
        [
            "acceptance",
            "discord",
            "acceptable-use",
            "server-admins",
            "paid-features",
            "warranty",
            "termination",
            "changes",
            "governing-law",
        ],
        ids.as_slice(),
    );
}

#[tokio::test]
async fn terms_markup_counts_and_links() {
    let html = page_html("/terms").await.unwrap();
    let main = between(&html, LEGAL_MAIN, "</main>").unwrap();

    assert_eq!(2, count(main, "<strong>"));
    assert_eq!(1, count(main, "<strong>kilooscarsix@gmail.com</strong>"));
    assert_eq!(3, count(main, "<ul>"));
    assert_eq!(19, count(main, "<li>"));
    assert_eq!(
        [
            "#acceptance",
            "#discord",
            "#acceptable-use",
            "#server-admins",
            "#paid-features",
            "#warranty",
            "#termination",
            "#changes",
            "#governing-law",
            "/privacy",
            "https://www.youtube.com/t/terms",
            "https://policies.google.com/privacy",
            "https://discord.com/terms",
            "https://discord.com/guidelines",
        ],
        hrefs(main).as_slice(),
    );
    assert!(!main.contains("target="));
    assert!(strip_tags(main).contains(
        "By using the YouTube features, you agree to be bound by the YouTube Terms of Service."
    ));
}

#[tokio::test]
async fn legal_pages_share_the_public_chrome() {
    for path in ["/privacy", "/terms"] {
        let html = page_html(path).await.unwrap();

        assert_eq!(1, count(&html, "<header class=\"public-header\">"), "{path}");
        assert_eq!(1, count(&html, "<footer class=\"footer\">"), "{path}");
        assert_eq!(1, count(&html, "<main"), "{path}");
        assert_eq!(1, texts(&html, "h1").len(), "{path}");
    }
}

#[tokio::test]
async fn privacy_carries_the_youtube_disclosures() {
    let html = page_html("/privacy").await.unwrap();
    let text = strip_tags(between(&html, LEGAL_MAIN, "</main>").unwrap());

    for required in [
        "Zayden uses YouTube API Services.",
        "channels.list?mine=true",
        "Zayden's use and transfer of information received from Google APIs will adhere to the",
        ", including the Limited Use requirements.",
    ] {
        assert!(text.contains(required), "privacy policy is missing {required}");
    }
}

#[tokio::test]
async fn terms_bind_youtube_users_to_the_youtube_terms() {
    let html = page_html("/terms").await.unwrap();
    let text = strip_tags(between(&html, LEGAL_MAIN, "</main>").unwrap());

    assert!(text.contains("By using the YouTube features, you agree to be bound by the YouTube Terms of Service"));
}

#[tokio::test]
async fn privacy_states_the_guild_retention_window() {
    let html = page_html("/privacy").await.unwrap();
    let window = format!("kept for {} days", zayden_app::guilds::RETENTION_DAYS);

    assert!(strip_tags(&html).contains(&window), "missing {window}");
}

/// Seeded so the account menu never asks Discord for the signed-in user.
async fn seed_users(users: &SessionUsersCache, ids: &[i64]) {
    for &id in ids {
        users
            .insert(id, SessionUser {
                id: id.to_string(),
                name: format!("User {id}"),
                avatar: None,
            })
            .await;
    }
}

#[tokio::test]
async fn every_public_page_has_its_own_meta_description() {
    let mut seen = Vec::new();
    for path in ["/", "/login", "/upgrade", "/privacy", "/terms"] {
        let html = page_html(path).await.unwrap();
        let head = between(&html, "<head>", "</head>").unwrap();
        let metas = head.matches(r#"<meta name="description" content=""#).count();
        assert_eq!(metas, 1, "{path}: {head}");
        let description =
            between(head, r#"<meta name="description" content=""#, "\"")
                .unwrap()
                .to_owned();
        assert!(description.len() > 40, "{path}: {description}");
        assert!(!seen.contains(&description), "{path} repeats {description}");
        seen.push(description);
    }

    let router = app(Some(web_state(Some(INVITE)).await.unwrap())).unwrap();
    let missing = body_text(get(&router, "/does-not-exist", None).await.unwrap())
        .await
        .unwrap();
    assert!(!missing.contains(r#"<meta name="description""#), "{missing}");
}
