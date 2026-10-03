//! The app frame and its pages through the app router, against Postgres.
//!
//! Sessions and Discord guild lists come from seeded caches, so no case
//! reaches Discord. Each `#[sqlx::test]` builds one full app state and runs
//! its scenarios in sequence. Pages hide or replace what the database fails
//! to load (tier, role links), so every such value is read directly before
//! the markup is checked: a database fault then fails with its own error.

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use http_body_util::BodyExt;
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use topcoat::Result as ViewResult;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::context::Cx;
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, StatusCode, header, page, path_param};
use topcoat::view::{View, view};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use web::auth::{WebRole, has_role};
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::shell::GuildId;
use web::state::{SessionIdentity, WebState};
use zayden_app::config::BotConfig;
use zayden_app::entitlement::{EntitlementScope, Tier};
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const MEMBER: &str = "session=member-token";
const STAFF: &str = "session=staff-token";
const PRO: &str = "session=pro-token";
const UPGRADE_URL: &str = "https://upgrade.example/pro";
const FORM: &str = "application/x-www-form-urlencoded";

/// `sqlx::test` lends every test pool's connections out of one 20-permit
/// master pool, and a pool keeps each permit it took until the pool is
/// dropped. One page fans out more queries than a default test pool's five
/// connections, so eight parallel tests ask for 40 permits and the starved
/// queries time out after 30 s. Two connections per pool keep this binary
/// under the budget.
const TEST_POOL_CONNECTIONS: u32 = 2;

const ICON_OPEN: &str = r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">"#;
const CHEVRON_DOWN: &str = r#"<path d="m6 9 6 6 6-6"/></svg>"#;
const CHECK: &str = r#"<path d="M20 6 9 17l-5-5"/></svg>"#;

path_param!(probe);

#[page("/guild/{guild_id}/{probe}")]
async fn guild_probe(cx: &Cx) -> ViewResult<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let probe: &str = path_param::<Probe>(cx);
    Ok(view! {
        <p id="probe">
            (guild_id)
            "/"
            (probe)
        </p>
    })
}

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-shell-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let assets = [
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
    ];
    let mut manifest = String::from("version = 1\n");
    for (file, content_type, id) in assets {
        std::fs::write(dir.join(file), "")?;
        write!(
            manifest,
            "\n[[assets]]\nid = {id}\nfile = \"{file}\"\nhash = \"0\"\ncontent_type = \"{content_type}\"\n"
        )?;
    }
    std::fs::write(dir.join("manifest.toml"), manifest)?;
    Ok(dir)
}

fn config(upgrade_url: Option<&str>) -> BotConfig {
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
        invite_url: None,
        upgrade_url: upgrade_url.map(str::to_owned),
        kofi_verification_token: None,
        patreon: None,
        youtube: None,
        discord_sku_pro: None,
        discord_sku_ultra: None,
        radio_stations: Arc::from(Vec::new()),
    }
}

fn guild(
    id: u64,
    name: &str,
    icon: Option<&str>,
    permissions: Permissions,
) -> TestResult<CurrentUserGuild> {
    Ok(CurrentUserGuild {
        id: Id::new(id),
        name: name.to_owned(),
        icon: icon.map(str::parse).transpose()?,
        owner: false,
        permissions,
        features: Vec::new(),
    })
}

async fn test_pool(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) -> TestResult<PgPool> {
    Ok(options.max_connections(TEST_POOL_CONNECTIONS).connect_with(connect).await?)
}

struct Harness {
    router: Router,
    app: Arc<ZaydenAppState>,
}

async fn web_state(
    pool: PgPool,
    upgrade_url: Option<&str>,
) -> TestResult<(WebState, Arc<ZaydenAppState>)> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let config = config(upgrade_url);
    let app = Arc::new(ZaydenAppState::new(pool, &config));
    let state = WebState::new(Arc::clone(&app), &config)?;

    for (token, user_id) in
        [("member-token", 41), ("staff-token", 42), ("pro-token", 44)]
    {
        state
            .sessions
            .insert(token.to_owned(), SessionIdentity {
                user_id,
                access_token: format!("{token}-access"),
            })
            .await;
    }
    state
        .discord
        .user_guilds
        .insert(
            41,
            Arc::from([
                guild(7, "Guild 7", None, Permissions::MANAGE_GUILD)?,
                guild(8, "Guild 8", None, Permissions::SEND_MESSAGES)?,
                guild(
                    9,
                    "Nine",
                    Some("a_0123456789abcdef0123456789abcdef"),
                    Permissions::ADMINISTRATOR,
                )?,
            ]),
        )
        .await;
    for user_id in [42, 44] {
        state.discord.user_guilds.insert(user_id, Arc::from([])).await;
    }

    Ok((state, app))
}

fn app_router(state: WebState) -> TestResult<Router> {
    let base = Router::builder()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .app_context(state)
        .page(guild_probe);
    Ok(web::router(base))
}

async fn harness(pool: &PgPool, upgrade_url: Option<&str>) -> TestResult<Harness> {
    let (state, app) = web_state(pool.clone(), upgrade_url).await?;
    Ok(Harness { router: app_router(state)?, app })
}

impl Harness {
    async fn get(&self, path: &str, cookie: Option<&str>) -> TestResult<Response> {
        let mut request = Request::get(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        Ok(self.router.handle(request.body(Body::empty())?).await)
    }

    async fn post(
        &self,
        path: &str,
        body: &str,
        cookie: Option<&str>,
    ) -> TestResult<Response> {
        let mut request = Request::post(path).header(header::CONTENT_TYPE, FORM);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        Ok(self.router.handle(request.body(Body::from(body.to_owned()))?).await)
    }

    async fn page(&self, path: &str, cookie: Option<&str>) -> TestResult<String> {
        let response = self.get(path, cookie).await?;
        if response.status() != StatusCode::OK {
            return Err(format!("{path} answered {}", response.status()).into());
        }
        body_text(response).await
    }
}

async fn body_text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(normalize(&String::from_utf8(bytes.to_vec())?))
}

/// Drops comments and runtime binding attributes, which the browser does not
/// show.
fn normalize(html: &str) -> String {
    let html = strip(html, "<!--", |rest| {
        rest.split_once("-->").map_or("", |(_, tail)| tail)
    });
    strip(&html, " data-topcoat-", |rest| {
        rest.split_once("=\"")
            .and_then(|(_, value)| value.split_once('"'))
            .map_or("", |(_, tail)| tail)
    })
}

fn strip<'a>(
    html: &'a str,
    start: &str,
    skip: impl Fn(&'a str) -> &'a str,
) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some((before, after)) = rest.split_once(start) {
        out.push_str(before);
        rest = skip(after);
    }
    out.push_str(rest);
    out
}

fn location(response: &Response) -> Option<&str> {
    response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok())
}

fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

async fn grant_role(pool: &PgPool, user_id: i64, role: &str) -> TestResult {
    sqlx::query!(
        "INSERT INTO web_user_roles (discord_user_id, role) VALUES ($1, $2)",
        user_id,
        role
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn sidebar_link(
    href: &str,
    icon: &str,
    label: &str,
    current: bool,
    active: bool,
) -> String {
    let aria = if current { r#" aria-current="page""# } else { "" };
    let class = if active { "app-sidebar-link active" } else { "app-sidebar-link" };
    format!(
        r#"<a href="{href}"{aria} class="{class}">{ICON_OPEN}{icon}<span>{label}</span></a>"#
    )
}

const SERVER: &str = r#"<rect width="20" height="8" x="2" y="2" rx="2"/><rect width="20" height="8" x="2" y="14" rx="2"/><line x1="6" x2="6.01" y1="6" y2="6"/><line x1="6" x2="6.01" y1="18" y2="18"/></svg>"#;
const ZAP: &str = r#"<path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"/></svg>"#;
const LEGAL: &str = r#"<nav class="legal-links" aria-label="Legal"><a href="/privacy">Privacy Policy</a><a href="/terms">Terms of Service</a></nav>"#;

const SHIELD: &str = r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/></svg>"#;
const GAMEPAD: &str = r#"<line x1="6" x2="10" y1="11" y2="11"/><line x1="8" x2="8" y1="9" y2="13"/><line x1="15" x2="15.01" y1="12" y2="12"/><line x1="18" x2="18.01" y1="10" y2="10"/><path d="M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z"/></svg>"#;
const SUBSCRIBE: &str = r#"<div class="upgrade-actions"><a href="/invite" rel="external" class="btn btn-secondary">Subscribe via Discord</a></div>"#;

#[sqlx::test(migrations = "../migrations")]
async fn the_guild_list_frames_the_managed_guilds(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    grant_role(&pool, 42, "operator").await.unwrap();
    grant_role(&pool, 42, "admin").await.unwrap();
    let app = harness(&pool, Some(UPGRADE_URL)).await.unwrap();

    assert!(!has_role(&pool, 41, WebRole::Admin).await.unwrap());
    assert!(!has_role(&pool, 41, WebRole::Operator).await.unwrap());
    let html = app.page("/guilds", Some(MEMBER)).await.unwrap();

    assert!(html.contains("<title>Servers - Zayden Dashboard</title>"), "{html}");
    assert!(html.contains(
        r#"<body><div class="app"><nav class="app-navbar"><a href="/guilds" aria-current="page" class="brand"><span class="brand-mark">Z</span>Zayden</a><div class="app-navbar-links">"#
    ), "{html}");
    assert!(
        html.contains(&format!(
            r#"<a href="/logout" rel="external" class="btn btn-ghost">{ICON_OPEN}"#
        )),
        "{html}"
    );
    assert!(
        html.contains(r#"<span class="tier-badge tier-free">Free</span>"#),
        "{html}"
    );
    assert!(html.contains(&format!(
        r#"<a href="{UPGRADE_URL}" class="btn-upgrade" target="_blank" rel="noopener noreferrer">Upgrade to Pro</a>"#
    )), "{html}");
    assert!(html.contains(&format!(
        r#"<div class="app-body"><aside class="app-sidebar"><div class="app-sidebar-heading">Dashboard</div>{}{}{LEGAL}</aside><main class="app-main"><div class="page"><div class="page-header"><div><h1>Your Servers</h1><p class="page-lead">Pick a server to configure Zayden.</p></div><a href="/invite" rel="external" class="btn btn-secondary">Add to a server</a></div><div class="guild-grid">"#,
        sidebar_link("/guilds", SERVER, "Servers", true, true),
        sidebar_link("/upgrade", ZAP, "Upgrade to Pro", false, false),
    )), "{html}");
    assert!(html.contains(
        r#"<a href="/guild/7" class="guild-card"><span class="guild-icon placeholder">G</span><div class="guild-card-body"><div class="guild-name">Guild 7</div>"#
    ), "{html}");
    assert!(html.contains(
        r#"<a href="/guild/9" class="guild-card"><img src="https://cdn.discordapp.com/icons/9/a_0123456789abcdef0123456789abcdef.png?size=64" alt="" class="guild-icon">"#
    ), "{html}");
    assert!(!html.contains("Guild 8"), "{html}");
    assert!(html.contains("</div></div></main></div></div></body>"), "{html}");
    assert!(!html.contains("server-switcher"));

    assert!(has_role(&pool, 42, WebRole::Admin).await.unwrap());
    assert!(has_role(&pool, 42, WebRole::Operator).await.unwrap());
    let html = app.page("/guilds", Some(STAFF)).await.unwrap();
    let links = [
        sidebar_link("/guilds", SERVER, "Servers", true, true),
        sidebar_link("/admin/servers", SHIELD, "All bot servers", false, false),
        sidebar_link(
            "/admin/destiny2/loadouts",
            GAMEPAD,
            "Loadout builder",
            false,
            false,
        ),
        sidebar_link("/upgrade", ZAP, "Upgrade to Pro", false, false),
    ];
    assert!(html.contains(&links.concat()), "{html}");
    assert!(
        html.contains(
            r#"<p class="empty">You manage no servers with this account.</p>"#
        ),
        "{html}"
    );

    for path in ["/guilds", "/guild/7", "/guild/7/probe", "/guild/abc"] {
        let response = app.get(path, None).await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(location(&response), Some("/login"), "{path}");
    }
    let response = app
        .post("/guild/7", "guild=7&module_id=ai&enabled=true", None)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));
    let response = app.get("/guilds", Some("session=unknown")).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn the_guild_frame_wraps_the_overview_and_nested_pages(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool, Some(UPGRADE_URL)).await.unwrap();
    app.app.modules.set(7, "gambling", true).await.unwrap();

    assert_eq!(
        app.app.modules.states(7).await.unwrap().get("gambling").copied(),
        Some(true)
    );
    assert!(!app.app.settings.ai.get(7).await.unwrap().enabled);
    assert!(!has_role(&pool, 41, WebRole::Operator).await.unwrap());
    let html = app.page("/guild/7", Some(MEMBER)).await.unwrap();

    assert!(html.contains("<title>Modules - Zayden Dashboard</title>"), "{html}");
    assert!(html.contains(
        r#"<a href="/guilds" class="brand"><span class="brand-mark">Z</span>Zayden</a>"#
    ), "{html}");
    assert!(html.contains(&format!(
        r#"<div class="app-body"><aside class="app-sidebar"><details class="server-switcher"><summary><span class="server-switcher-avatar placeholder">G</span><span class="server-switcher-name">Guild 7</span>{ICON_OPEN}{CHEVRON_DOWN}</summary><div class="server-switcher-menu"><a href="/guild/7" aria-current="page" class="server-switcher-option current"><span class="server-switcher-avatar placeholder">G</span><span class="server-switcher-name">Guild 7</span>{ICON_OPEN}{CHECK}</a><a href="/guild/9" class="server-switcher-option"><img src="https://cdn.discordapp.com/icons/9/a_0123456789abcdef0123456789abcdef.png?size=64" alt="" class="server-switcher-avatar"><span class="server-switcher-name">Nine</span></a></div></details><div class="app-sidebar-heading">Manage</div><div class="app-sidebar-group"><div class="app-sidebar-group-head"><a href="/guild/7" aria-current="page" class="app-sidebar-link active">"#
    )), "{html}");
    assert!(!html.contains("operator-badge"), "{html}");
    assert!(html.contains(&format!(
        r#"<span>Modules</span></a><button type="button" class="app-sidebar-caret open" aria-label="Toggle module list" aria-expanded="true">{ICON_OPEN}{CHEVRON_DOWN}</button></div><div class="app-sidebar-sublist open"><a href="/guild/7/settings/general" class="app-sidebar-sublink">General</a><a href="/guild/7/settings/ai" class="app-sidebar-sublink">AI Chat</a>"#
    )), "{html}");
    assert!(html.contains(
        r#"<a href="/guild/7/settings/youtube" class="app-sidebar-sublink">YouTube</a></div></div><div class="app-sidebar-spacer"></div>"#
    ), "{html}");
    assert_eq!(count(&html, r#"class="app-sidebar-sublink""#), 13, "{html}");
    assert!(
        html.contains(&sidebar_link("/guilds", SERVER, "All servers", false, false)),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            "{}{LEGAL}</aside>",
            sidebar_link("/upgrade", ZAP, "Upgrade to Pro", false, false)
        )),
        "{html}"
    );
    assert!(html.contains(
        r#"<main class="app-main"><div class="page"><div class="page-header"><div><h1>Modules</h1><p class="page-lead">Turn modules on or off for this server. A module that's off has its commands removed from the server.</p></div><a href="/guild/7/settings/general" class="btn btn-secondary">Server settings</a></div>"#
    ), "{html}");
    assert_eq!(count(&html, r#"<div class="module-card">"#), 15, "{html}");
    assert_eq!(
        count(&html, r#"<span class="module-status">Unknown</span>"#),
        11,
        "{html}"
    );
    assert!(html.contains(
        r#"<button class="toggle toggle-on" aria-label="Toggle module" aria-pressed="true" form="module-toggle-gambling" name="enabled" value="false"></button>"#
    ), "{html}");
    assert!(html.contains(
        r#"<button class="toggle" aria-label="Toggle module" aria-pressed="false" form="module-toggle-ai" name="enabled" value="true"></button>"#
    ), "{html}");
    assert_eq!(
        count(
            &html,
            "This module is switched on from its own settings page \u{2014} use Configure below."
        ),
        2,
        "{html}"
    );
    assert_eq!(count(&html, r#"class="module-configure""#), 8, "{html}");
    assert!(html.contains(r#"<form id="module-toggle-ai" method="post" action="/guild/7"><input type="hidden" name="guild" value="7"><input type="hidden" name="module_id" value="ai"></form>"#), "{html}");

    let html = app.page("/guild/7/probe", Some(MEMBER)).await.unwrap();
    assert!(
        html.contains(
            r#"<a href="/guild/7" aria-current="page" class="app-sidebar-link">"#
        ),
        "{html}"
    );
    assert!(
        html.contains(r#"<main class="app-main"><p id="probe">7/probe</p></main>"#),
        "{html}"
    );
    assert_eq!(count(&html, "app-sidebar-sublink active"), 0, "{html}");

    let html = app.page("/guild/7/settings", Some(MEMBER)).await.unwrap();
    assert!(
        html.contains(
            r#"<a href="/guild/7/settings/general" class="app-sidebar-sublink active">General</a>"#
        ),
        "{html}"
    );
    assert_eq!(count(&html, "app-sidebar-sublink active"), 1, "{html}");

    let html = app.page("/guild/abc", Some(MEMBER)).await.unwrap();
    assert!(html.contains(r#"<p class="error">Failed to load modules: error running server function: invalid guild id</p>"#), "{html}");
    assert!(!html.contains("server-switcher"), "{html}");
    assert!(html.contains(r#"<a href="/guild/abc" aria-current="page" class="app-sidebar-link active">"#), "{html}");

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn module_toggles_save_or_report_why_not(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool, Some(UPGRADE_URL)).await.unwrap();

    let response = app
        .post("/guild/7", "guild=7&module_id=ai&enabled=true", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/guild/7"));
    assert!(app.app.settings.ai.get(7).await.unwrap().enabled);
    let html = app.page("/guild/7", Some(MEMBER)).await.unwrap();
    assert!(html.contains(
        r#"<button class="toggle toggle-on" aria-label="Toggle module" aria-pressed="true" form="module-toggle-ai" name="enabled" value="false"></button>"#
    ), "{html}");

    let response = app
        .post("/guild/7", "guild=7&module_id=gambling&enabled=true", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        app.app.modules.states(7).await.unwrap().get("gambling").copied(),
        Some(true)
    );
    let html = app.page("/guild/7", Some(MEMBER)).await.unwrap();
    assert!(
        html.contains(
            r#"form="module-toggle-gambling" name="enabled" value="false""#
        ),
        "{html}"
    );

    let response = app
        .post("/guild/7", "guild=7&module_id=patreon&enabled=true", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains("<title>Modules - Zayden Dashboard</title>"), "{html}");
    assert!(html.contains(r#"<details class="server-switcher">"#), "{html}");
    assert!(html.contains(
        r#"<p class="module-error">error running server function: Patreon is switched on from its own settings page, not from this toggle.</p><div class="module-card-foot"><span class="module-status failed">Not saved</span>"#
    ), "{html}");
    assert_eq!(count(&html, "module-error"), 1, "{html}");

    let response = app
        .post("/guild/7", "guild=8&module_id=ai&enabled=false", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"<p class="module-error">error running server function: invalid value for `guild`</p>"#), "{html}");
    assert!(app.app.settings.ai.get(7).await.unwrap().enabled);

    let response = app
        .post("/guild/7", "guild=7&module_id=ai&enabled=false&extra=1", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"<p class="module-error">error running server function: unknown field `extra`</p>"#), "{html}");

    for (body, message) in [
        ("guild=7&enabled=true", "missing field `module_id`"),
        ("guild=7&module_id=bogus&enabled=true", "unknown module"),
    ] {
        let response = app.post("/guild/7", body, Some(MEMBER)).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let html = body_text(response).await.unwrap();
        assert!(
            html.contains(&format!(
                r#"<p class="error">error running server function: {message}</p><div class="module-grid">"#
            )),
            "{html}"
        );
        assert_eq!(count(&html, "module-error"), 0, "{html}");
    }

    let response = app
        .post("/guild/8", "guild=8&module_id=ai&enabled=true", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"<p class="error">Failed to load modules: error running server function: forbidden</p>"#), "{html}");
    assert!(!html.contains("server-switcher"), "{html}");

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn the_upgrade_page_follows_the_viewer_and_links_kofi_emails(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool, Some(UPGRADE_URL)).await.unwrap();

    let html = app.page("/upgrade", None).await.unwrap();
    assert!(html.contains("<title>Upgrade - Zayden Dashboard</title>"), "{html}");
    assert!(html.contains(r#"<a href="/auth/discord" rel="external" class="btn btn-primary">Sign in</a>"#), "{html}");
    assert!(!html.contains("tier-badge tier-free"), "{html}");
    assert!(
        html.contains(&sidebar_link("/upgrade", ZAP, "Upgrade to Pro", true, true)),
        "{html}"
    );
    assert!(
        html.contains(&sidebar_link("/guilds", SERVER, "Servers", false, false)),
        "{html}"
    );
    assert!(html.contains(r#"<h1>Upgrade your plan</h1><p class="page-lead">Paid tiers are cost-recovery: they unlock the features that cost real money to run. Everything else stays free.</p>"#), "{html}");
    assert!(html.contains(
        "<strong>Lower greeting cooldowns</strong><span>Drop the per-member and server-wide limits on <code>/good</code> - for servers busy enough to hit them.</span>"
    ), "{html}");
    assert!(html.contains(&format!(
        r#"<div class="plan-ladder"><div class="plan-card plan-pro"><div class="plan-head"><span class="tier-badge tier-pro">Pro</span><span class="plan-price">$2.99<small>/mo</small></span></div><ul class="plan-specs"><li><strong>50 MB</strong> Palworld save uploads</li><li><strong>30 min</strong> upload cooldown</li></ul><a href="{UPGRADE_URL}" class="btn btn-primary" target="_blank" rel="noopener noreferrer">Get Pro</a></div></div>{SUBSCRIBE}"#
    )), "{html}");
    assert!(html.contains(
        r#"<div class="card"><p class="label">Link your Ko-fi email</p><p class="page-lead">Connect the email you subscribe with on Ko-fi so your paid membership follows your Discord account.</p><form method="post" action="/upgrade" data-pending=""><div class="kofi-link-form"><input class="input" type="email" name="email" placeholder="you@example.com" required=""><button type="submit" class="btn btn-primary">Link email</button></div></form></div>"#
    ), "{html}");

    app.app
        .entitlements
        .grant(EntitlementScope::User(44), Tier::Pro, "test", "pro-44", None)
        .await
        .unwrap();
    assert_eq!(app.app.entitlements.user_tier(44).await, Tier::Pro);
    let html = app.page("/upgrade", Some(PRO)).await.unwrap();
    assert!(
        html.contains(r#"<span class="tier-badge tier-pro">Pro</span>"#),
        "{html}"
    );
    assert!(!html.contains("btn-upgrade"), "{html}");
    assert!(
        html.contains(r#"</ul><span class="plan-current">Your plan</span></div>"#),
        "{html}"
    );
    assert!(!html.contains("Get Pro"), "{html}");

    let response =
        app.post("/upgrade", "email=fan%40example.com", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"value="fan@example.com"><button type="submit" class="btn btn-primary">Link email</button></div></form><p class="error">error running server function: unauthenticated</p></div>"#), "{html}");

    let response =
        app.post("/upgrade", "email=Fan%40Example.com", Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"value="Fan@Example.com"><button type="submit" class="btn btn-primary">Link email</button></div></form><p class="success">Ko-fi email linked.</p></div>"#), "{html}");
    assert!(html.contains("<title>Upgrade - Zayden Dashboard</title>"), "{html}");

    let response =
        app.post("/upgrade", "email=fan%40example.com", Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"<p class="error">error running server function: This Ko-fi email is already linked to an account.</p>"#), "{html}");

    let response = app.post("/upgrade", "email=nope", Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(
        html.contains(
            r#"<p class="error">error running server function: invalid email</p>"#
        ),
        "{html}"
    );

    let unlinked = harness(&pool, None).await.unwrap();
    let html = unlinked.page("/upgrade", None).await.unwrap();
    assert!(
        html.contains(r#"<ul class="plan-specs"><li><strong>50 MB</strong> Palworld save uploads</li><li><strong>30 min</strong> upload cooldown</li></ul></div></div>"#),
        "{html}"
    );
    assert!(html.contains(SUBSCRIBE), "{html}");
    assert!(!html.contains("Get Pro"), "{html}");
    let html = unlinked.page("/guilds", Some(MEMBER)).await.unwrap();
    assert!(
        html.contains(r#"<span class="tier-badge tier-free">Free</span>"#),
        "{html}"
    );
    assert!(!html.contains("btn-upgrade"), "{html}");

    pool.close().await;
}

#[tokio::test]
async fn an_unreachable_database_leaves_out_what_it_cannot_load() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy("postgres://127.0.0.1:1/unused")
        .unwrap();
    let (state, app) = web_state(pool, Some(UPGRADE_URL)).await.unwrap();
    let app = Harness { router: app_router(state).unwrap(), app };
    let cookie = Some("session=uncached-token");

    let html = app.page("/upgrade", cookie).await.unwrap();
    assert!(!html.contains("Log out"), "{html}");
    assert!(!html.contains("Sign in"), "{html}");
    assert!(!html.contains("tier-badge tier-"), "{html}");
    assert!(!html.contains("plan-ladder"), "{html}");
    assert!(!html.contains(SUBSCRIBE), "{html}");
    assert!(html.contains(&format!(
        r#"<aside class="app-sidebar"><div class="app-sidebar-heading">Dashboard</div>{}{}{LEGAL}</aside>"#,
        sidebar_link("/guilds", SERVER, "Servers", false, false),
        sidebar_link("/upgrade", ZAP, "Upgrade to Pro", true, true),
    )), "{html}");
    assert!(
        html.contains(r#"<p class="label">Link your Ko-fi email</p>"#),
        "{html}"
    );

    let html = app.page("/guilds", cookie).await.unwrap();
    assert!(
        html.contains(
            r#"<p class="error">Failed to load servers: error running server function: "#
        ),
        "{html}"
    );
}
