//! The app frame and its pages through the app router, against Postgres.
//!
//! Sessions, Discord users and guild lists come from seeded caches, and the
//! bot's Discord client talks to a local stand-in that rejects its token the
//! way Discord does, so no case reaches Discord. Each `#[sqlx::test]` builds one
//! full app state and runs its scenarios in sequence. Pages hide or replace what the
//! database fails to load (tier, role links), so every such value is read directly
//! before the markup is checked: a database fault then fails with its own error.

use std::error::Error;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

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
use web::auth::{SessionUser, WebRole, has_role};
use web::components::brand::LOGO;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::nav::{GENERAL, MODULES};
use web::shell::GuildId;
use web::state::{DiscordState, SessionIdentity, SessionUsersCache, WebState};
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
        ("logo-0123456789abcdef.png", "image/png", LOGO.id().as_u64()),
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

/// A local Discord API that answers every request with the 401 Discord gives
/// the test bot token. Returns its address.
fn unauthorized_discord() -> TestResult<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?.to_string();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                let _ = reject(stream);
            });
        }
    });
    Ok(addr)
}

fn reject(stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut stream = stream;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
    }

    let body = r#"{"message":"401: Unauthorized","code":0}"#;
    let response = format!(
        "HTTP/1.1 401 Unauthorized\r\ncontent-type: application/json\r\n\
         content-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
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
    let mut state = WebState::new(Arc::clone(&app), &config)?;
    state.discord = DiscordState {
        http: Arc::new(
            twilight_http::Client::builder()
                .token("test-bot-token".to_owned())
                .proxy(unauthorized_discord()?, true)
                .ratelimiter(None)
                .build(),
        ),
        user_guilds: state.discord.user_guilds,
        users: state.discord.users,
    };

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

    seed_users(&state.discord.users, &[41, 42, 44]).await;
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

fn nav_link(href: &str, icon: Option<&str>, label: &str, current: bool) -> String {
    let aria = if current { r#" aria-current="page""# } else { "" };
    let icon = icon.map_or_else(String::new, |icon| format!("{ICON_OPEN}{icon}"));
    format!(
        r#"<li><a href="{href}" class="nav-link"{aria}>{icon}<span>{label}</span></a></li>"#
    )
}

/// The dashboard links outside a guild: servers, plans and, for staff, the
/// admin group. The rail and the menu sheet each render them once.
fn outer_nav(servers: bool, plans: bool, admin: &str) -> String {
    format!(
        r#"<nav class="nav" aria-label="Dashboard"><ul class="nav-list">{}</ul><ul class="nav-list nav-group">{}</ul>{admin}</nav>"#,
        nav_link("/guilds", Some(SERVER), "Servers", servers),
        nav_link("/upgrade", Some(ZAP), "Plans", plans),
    )
}

const SERVER: &str = r#"<rect width="20" height="8" x="2" y="2" rx="2"/><rect width="20" height="8" x="2" y="14" rx="2"/><line x1="6" x2="6.01" y1="6" y2="6"/><line x1="6" x2="6.01" y1="18" y2="18"/></svg>"#;
const ZAP: &str = r#"<path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"/></svg>"#;
const GRID: &str = r#"<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/></svg>"#;
const SETTINGS: &str = r#"<line x1="21" x2="14" y1="4" y2="4"/><line x1="10" x2="3" y1="4" y2="4"/><line x1="21" x2="12" y1="12" y2="12"/><line x1="8" x2="3" y1="12" y2="12"/><line x1="21" x2="16" y1="20" y2="20"/><line x1="12" x2="3" y1="20" y2="20"/><line x1="14" x2="14" y1="2" y2="6"/><line x1="8" x2="8" y1="10" y2="14"/><line x1="16" x2="16" y1="18" y2="22"/></svg>"#;
const MAIN: &str = r#"<main id="main" class="app-main" tabindex="-1">"#;
const FREE_CHIP: &str = r#"<a href="/upgrade" class="plan-chip" aria-label="Upgrade, current plan: Free">Upgrade</a>"#;
const LEGAL: &str = r#"<nav class="legal-links" aria-label="Legal"><a href="/privacy">Privacy Policy</a><a href="/terms">Terms of Service</a></nav>"#;

const SHIELD: &str = r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/></svg>"#;
const GAMEPAD: &str = r#"<line x1="6" x2="10" y1="11" y2="11"/><line x1="8" x2="8" y1="9" y2="13"/><line x1="15" x2="15.01" y1="12" y2="12"/><line x1="18" x2="18.01" y1="10" y2="10"/><path d="M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z"/></svg>"#;
const SUBSCRIBE: &str = r#"<div class="upgrade-actions"><p class="page-lead">Prefer to pay through Discord? Add Zayden to a server first, then subscribe from Zayden's profile in Discord.</p><a href="/invite" rel="external" class="btn btn-secondary">Add Zayden to Discord</a></div>"#;
const OVERVIEW_HEADER: &str = r#"<div class="page"><div class="page-header"><div><h1>Overview</h1><p class="page-lead">Turn modules on or off for this server. A module that's off has its commands removed from the server.</p></div><a href="/guild/7/settings" class="btn btn-secondary">Server settings</a></div>"#;
const ROW: &str = r#"<li class="rack-row">"#;
const NOT_SYNCED: &str =
    r#"<span class="lamp lamp-sync" aria-hidden="true"></span>"#;

/// The `name=value` pair of the flash cookie a response sets.
fn flash_cookie(response: &Response) -> Option<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("flash=") && !value.starts_with("flash=;"))
        .and_then(|value| value.split(';').next())
        .map(str::to_owned)
}

/// The rack switch of a command module, in its posting form.
fn rack_switch(module_id: &str, label: &str, on: bool) -> String {
    let (lamp, text, checked, wanted) = if on {
        ("on", "On", "true", "false")
    } else {
        ("off", "Off", "false", "true")
    };
    format!(
        r#"<form class="rack-form" method="post" action="/guild/7" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="module_id" value="{module_id}"><div class="rack-state"><span class="lamp-status"><span class="lamp lamp-{lamp}" aria-hidden="true"></span><span class="lamp-text" data-pending-text="Saving…">{text}</span></span></div><div class="rack-control"><button type="submit" class="switch" role="switch" aria-checked="{checked}" aria-label="{label} module" name="enabled" value="{wanted}"><span class="switch-thumb" aria-hidden="true"></span></button></div></form>"#
    )
}

/// The status and link a module switched on its own page shows in the rack.
fn rack_status(module_path: &str, on: bool) -> String {
    let (lamp, text) = if on { ("on", "On") } else { ("off", "Off") };
    format!(
        r#"<div class="rack-state"><span class="lamp-status"><span class="lamp lamp-{lamp}" aria-hidden="true"></span><span class="lamp-text">{text}</span></span></div><div class="rack-control"><a href="/guild/7/{module_path}" class="rack-link">Manage{ICON_OPEN}<path d="m9 18 6-6-6-6"/></svg></a></div>"#
    )
}

/// The result line a rack group shows after a switch.
fn group_flash(group: &str, message: &str) -> String {
    format!(
        r#"<div id="modules-{group}"><div class="flash-region" role="status"><p class="flash flash-success" data-flash="">{ICON_OPEN}{CHECK}<span class="flash-text">{message}</span></p></div><section class="rack-group""#
    )
}

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
        r##"<body><a class="skip-link" href="#main">Skip to main content</a><div class="app"><header class="app-header"><div class="topbar"><button type="button" class="topbar-button menu-button" popovertarget="nav-sheet" aria-controls="nav-sheet" aria-expanded="false" aria-label="Menu">"##
    ), "{html}");
    assert!(html.contains(
        r#"<a href="/guilds" class="brand"><img class="brand-mark" src="/_topcoat/assets/logo-0123456789abcdef.png" alt="" width="28" height="28"><span class="brand-name">Zayden</span></a>"#
    ), "{html}");
    assert!(
        html.contains(&format!(
            r#"<a href="/logout" rel="external" class="menu-item">{ICON_OPEN}"#
        )),
        "{html}"
    );
    assert!(html.contains(FREE_CHIP), "{html}");
    let nav = outer_nav(true, false, "");
    assert_eq!(count(&html, &nav), 2, "{html}");
    assert!(
        html.contains(&format!(
            r#"<div class="rail">{nav}<div class="nav-spacer"></div>{LEGAL}</div>"#
        )),
        "{html}"
    );
    assert!(html.contains(&format!(
        r#"</header>{MAIN}<div class="page"><div class="page-header"><div><h1>Servers</h1><p class="page-lead">Pick a server to configure Zayden.</p></div><a href="/invite" rel="external" class="btn btn-secondary">Add Zayden to a server</a></div><div class="guild-grid">"#
    )), "{html}");
    assert!(html.contains(
        r#"<a href="/guild/7" class="guild-card"><span class="guild-icon placeholder">G</span><div class="guild-card-body"><div class="guild-name">Guild 7</div>"#
    ), "{html}");
    assert!(html.contains(
        r#"<a href="/guild/9" class="guild-card"><img src="https://cdn.discordapp.com/icons/9/a_0123456789abcdef0123456789abcdef.png?size=64" alt="" class="guild-icon">"#
    ), "{html}");
    assert!(!html.contains("Guild 8"), "{html}");
    assert!(html.contains("</div></div></main></div></body>"), "{html}");
    assert!(!html.contains("server-switcher"));

    assert!(has_role(&pool, 42, WebRole::Admin).await.unwrap());
    assert!(has_role(&pool, 42, WebRole::Operator).await.unwrap());
    let html = app.page("/guilds", Some(STAFF)).await.unwrap();
    for prefix in ["rail", "sheet"] {
        let admin = format!(
            r#"<div class="nav-group"><p class="nav-heading" id="{prefix}-admin">Admin</p><ul class="nav-list" aria-labelledby="{prefix}-admin">{}{}</ul></div>"#,
            nav_link("/admin/servers", Some(SHIELD), "All bot servers", false),
            nav_link(
                "/admin/destiny2/loadouts",
                Some(GAMEPAD),
                "Loadout builder",
                false
            ),
        );
        assert!(html.contains(&outer_nav(true, false, &admin)), "{html}");
    }
    assert!(
        html.contains(
            r#"<h1>Servers</h1><p class="page-lead">Pick a server to configure Zayden.</p></div></div><div class="empty-state"><h2 class="empty-title">Add Zayden to a server you manage</h2><p class="empty-text">Servers appear here when you have Manage Server in them.</p><a href="/invite" rel="external" class="btn btn-primary">Add Zayden to a server</a></div>"#
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

    assert!(html.contains("<title>Overview - Zayden Dashboard</title>"), "{html}");
    assert!(html.contains(
        r#"<a href="/guilds" class="brand"><img class="brand-mark" src="/_topcoat/assets/logo-0123456789abcdef.png" alt="" width="28" height="28"><span class="brand-name">Zayden</span></a>"#
    ), "{html}");
    assert!(html.contains(&format!(
        r#"<button type="button" class="topbar-button plate" popovertarget="server-switcher" aria-controls="server-switcher" aria-expanded="false" aria-label="Switch server, current: Guild 7"><span class="plate-avatar placeholder" aria-hidden="true">G</span><span class="plate-name">Guild 7</span>{ICON_OPEN}{CHEVRON_DOWN}</button>"#
    )), "{html}");
    assert!(html.contains(&format!(
        r#"<div class="menu-list" data-filter-list=""><a href="/guild/7" class="menu-item" aria-current="page" data-filter-item=""><span class="plate-avatar placeholder" aria-hidden="true">G</span><span class="plate-name">Guild 7</span>{ICON_OPEN}{CHECK}</a><a href="/guild/9" class="menu-item" data-filter-item=""><img src="https://cdn.discordapp.com/icons/9/a_0123456789abcdef0123456789abcdef.png?size=64" alt="" width="24" height="24" class="plate-avatar"><span class="plate-name">Nine</span></a></div><a href="/guilds" class="menu-item">{ICON_OPEN}{SERVER}<span class="plate-name">All servers</span></a>"#
    )), "{html}");
    assert!(!html.contains("operator-badge"), "{html}");
    assert!(html.contains(&format!(
        r#"<nav class="nav" aria-label="Dashboard"><ul class="nav-list">{}{}</ul><div class="nav-group"><p class="nav-heading" id="rail-community">Community</p><ul class="nav-list" aria-labelledby="rail-community">{}"#,
        nav_link("/guild/7", Some(GRID), "Overview", true),
        nav_link("/guild/7/settings", Some(SETTINGS), "Server settings", false),
        nav_link("/guild/7/greetings", None, "Greetings", false),
    )), "{html}");
    for heading in ["community", "support---safety", "voice---games", "integrations"]
    {
        for prefix in ["rail", "sheet"] {
            assert!(
                html.contains(&format!(r#"id="{prefix}-{heading}""#)),
                "{prefix}-{heading}: {html}"
            );
        }
    }
    for module in MODULES {
        let href = module.href(7);
        let icon = (*module == GENERAL).then_some(SETTINGS);
        assert_eq!(
            count(&html, &nav_link(&href, icon, module.label, false)),
            2,
            "{href}: {html}"
        );
    }
    assert_eq!(
        count(&html, &nav_link("/upgrade", Some(ZAP), "Plans", false)),
        2,
        "{html}"
    );
    assert!(
        html.contains(&format!(
            r#"{MAIN}{OVERVIEW_HEADER}<div class="flash-region" role="status"></div><div id="modules-community"><section class="rack-group" aria-labelledby="rack-community"><h2 class="rack-heading label" id="rack-community">Community</h2><ul class="rack"><li class="rack-row"><div class="rack-main"><h3 class="rack-name"><a href="/guild/7/greetings">Greetings</a></h3>"#
        )),
        "{html}"
    );
    for (group, heading, title) in [
        ("support", "support---safety", "Support &amp; safety"),
        ("voice", "voice---games", "Voice &amp; games"),
        ("integrations", "integrations", "Integrations"),
    ] {
        assert!(
            html.contains(&format!(
                r#"<div id="modules-{group}"><section class="rack-group" aria-labelledby="rack-{heading}"><h2 class="rack-heading label" id="rack-{heading}">{title}</h2><ul class="rack">"#
            )),
            "{group}: {html}"
        );
    }
    assert_eq!(count(&html, ROW), 15, "{html}");
    assert_eq!(count(&html, NOT_SYNCED), 11, "{html}");
    assert_eq!(count(&html, r#"role="switch""#), 1, "{html}");
    assert!(
        html.contains(&rack_switch("gambling", "Gambling &amp; Economy", true)),
        "{html}"
    );
    for path in ["ai", "patreon", "youtube"] {
        assert!(html.contains(&rack_status(path, false)), "{path}: {html}");
    }
    assert!(!html.contains("Toggle module"), "{html}");
    assert!(!html.contains("module-card"), "{html}");

    let html = app.page("/guild/7/probe", Some(MEMBER)).await.unwrap();
    assert!(
        html.contains(&nav_link("/guild/7", Some(GRID), "Overview", false)),
        "{html}"
    );
    assert!(
        html.contains(&format!(r#"{MAIN}<p id="probe">7/probe</p></main>"#)),
        "{html}"
    );
    assert_eq!(count(&html, r#"class="nav-link" aria-current="page""#), 0, "{html}");

    let html = app.page("/guild/7/settings", Some(MEMBER)).await.unwrap();
    let general =
        nav_link("/guild/7/settings", Some(SETTINGS), "Server settings", true);
    assert_eq!(count(&html, &general), 2, "{html}");
    assert_eq!(count(&html, r#"class="nav-link" aria-current="page""#), 2, "{html}");

    let html = app.page("/guild/abc", Some(MEMBER)).await.unwrap();
    assert!(html.contains(&format!(
        r#"{MAIN}<div class="page"><section class="error-panel"><h1 class="error-title">Server not found</h1><p class="error-text">That address doesn't name a Discord server.</p><div class="error-actions"><a href="/guilds" class="btn btn-primary">Back to servers</a></div></section></div></main>"#
    )), "{html}");
    assert!(!html.contains("server-switcher"), "{html}");
    assert!(
        html.contains(&nav_link("/guild/abc", Some(GRID), "Overview", true)),
        "{html}"
    );

    pool.close().await;
}

/// A `suspense` child that has taken the only connection is not polled again
/// until the rest of the page has its first content, so a chrome lookup beside
/// it would wait out the acquire timeout. One connection must be enough.
#[sqlx::test(migrations = "../migrations")]
async fn the_chrome_and_a_streamed_page_body_share_one_connection(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let acquire = Duration::from_secs(5);
    let pool = options
        .max_connections(1)
        .acquire_timeout(acquire)
        .connect_with(connect)
        .await
        .unwrap();
    grant_role(&pool, 41, "operator").await.unwrap();
    let app = harness(&pool, Some(UPGRADE_URL)).await.unwrap();

    for (path, loaded) in
        [("/guild/7", "All bot servers"), ("/upgrade", "plan-ladder")]
    {
        let started = Instant::now();
        let html = app.page(path, Some(MEMBER)).await.unwrap();
        assert!(started.elapsed() < acquire, "{path} took {:?}", started.elapsed());
        assert!(!html.contains("timed out"), "{path}: {html}");
        assert!(html.contains(loaded), "{path}: {html}");
    }
    let html = app.page("/guild/7", Some(MEMBER)).await.unwrap();
    assert_eq!(count(&html, ROW), 15, "{html}");

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
    assert_eq!(location(&response), Some("/guild/7#modules-integrations"));
    let flash = flash_cookie(&response).unwrap();
    assert!(app.app.settings.ai.get(7).await.unwrap().enabled);
    let html =
        app.page("/guild/7", Some(&format!("{MEMBER}; {flash}"))).await.unwrap();
    assert!(
        html.contains(&group_flash("integrations", "AI Chat turned on.")),
        "{html}"
    );
    assert!(html.contains(&rack_status("ai", true)), "{html}");
    assert_eq!(count(&html, "flash-text"), 1, "{html}");
    let html = app.page("/guild/7", Some(MEMBER)).await.unwrap();
    assert!(!html.contains("flash-text"), "{html}");

    let response = app
        .post("/guild/7", "guild=7&module_id=gambling&enabled=true", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/guild/7#modules-community"));
    let flash = flash_cookie(&response).unwrap();
    assert_eq!(
        app.app.modules.states(7).await.unwrap().get("gambling").copied(),
        Some(true)
    );
    let html =
        app.page("/guild/7", Some(&format!("{MEMBER}; {flash}"))).await.unwrap();
    assert!(
        html.contains(&group_flash(
            "community",
            "Gambling &amp; Economy turned on."
        )),
        "{html}"
    );
    assert!(
        html.contains(&rack_switch("gambling", "Gambling &amp; Economy", true)),
        "{html}"
    );

    let response = app
        .post("/guild/7", "guild=7&module_id=patreon&enabled=true", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains("<title>Overview - Zayden Dashboard</title>"), "{html}");
    assert!(html.contains(r#"aria-controls="server-switcher""#), "{html}");
    assert!(html.contains(&format!(
        r#"{}<p class="rack-error" role="alert">Not changed: Patreon is switched on from its own settings page, not from this toggle.</p></li>"#,
        rack_status("patreon", false)
    )), "{html}");
    assert_eq!(count(&html, "rack-error"), 1, "{html}");

    let response = app
        .post("/guild/7", "guild=8&module_id=ai&enabled=false", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"<p class="rack-error" role="alert">Not changed: invalid value for `guild`</p>"#), "{html}");
    assert!(app.app.settings.ai.get(7).await.unwrap().enabled);

    let response = app
        .post("/guild/7", "guild=7&module_id=ai&enabled=false&extra=1", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"<p class="rack-error" role="alert">Not changed: unknown field `extra`</p>"#), "{html}");

    for (body, message) in [
        ("guild=7&enabled=true", "missing field `module_id`"),
        ("guild=7&module_id=bogus&enabled=true", "unknown module"),
    ] {
        let response = app.post("/guild/7", body, Some(MEMBER)).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let html = body_text(response).await.unwrap();
        assert!(
            html.contains(&format!(
                r#"<div class="flash-region" role="status"></div><p class="error" role="alert">Not changed: {message}</p><div id="modules-community">"#
            )),
            "{html}"
        );
        assert_eq!(count(&html, "rack-error"), 0, "{html}");
    }

    let response = app
        .post("/guild/8", "guild=8&module_id=ai&enabled=true", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"<section class="error-panel"><h1 class="error-title">You can't manage this server</h1><p class="error-text">You need Manage Server in this server to change its modules.</p><div class="error-actions"><a href="/guilds" class="btn btn-primary">Back to servers</a></div></section>"#), "{html}");
    assert!(!html.contains("server-switcher"), "{html}");

    pool.close().await;
}

const KOFI_HELP: &str = "The email you pay with on Ko-fi. It is linked to the Discord account you are signed in with.";

/// The Ko-fi form; `email` is the typed value kept after a refusal and
/// `error` the reason shown at the field.
fn kofi_form(email: Option<&str>, error: Option<&str>) -> String {
    let value =
        email.map_or_else(String::new, |email| format!(r#" value="{email}""#));
    let (described, invalid, line) = error.map_or_else(
        || ("kofi-email-help".to_owned(), String::new(), String::new()),
        |error| {
            (
                "kofi-email-help kofi-email-error".to_owned(),
                r#" aria-invalid="true""#.to_owned(),
                format!(
                    r#"<p class="field-error" id="kofi-email-error">{error}</p>"#
                ),
            )
        },
    );
    let invalid_row = if error.is_some() { r#" data-invalid="""# } else { "" };
    format!(
        r#"<div class="field-row"{invalid_row}><label class="field-label" for="kofi-email">Ko-fi email</label><div class="kofi-link-form"><input class="input" id="kofi-email" type="email" name="email" autocomplete="email" placeholder="you@example.com" required=""{value} aria-describedby="{described}"{invalid}><button type="submit" class="btn btn-primary" data-pending-label="Linking…">Link email</button></div><p class="field-help" id="kofi-email-help">{KOFI_HELP}</p>{line}</div></form></section>"#
    )
}

#[sqlx::test(migrations = "../migrations")]
async fn the_upgrade_page_follows_the_viewer_and_links_kofi_emails(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool, Some(UPGRADE_URL)).await.unwrap();

    let html = app.page("/upgrade", None).await.unwrap();
    assert!(html.contains("<title>Plans - Zayden</title>"), "{html}");
    assert!(html.contains(r#"<header class="public-header">"#), "{html}");
    assert!(
        html.contains(r#"<a href="/auth/discord" rel="external" class="btn btn-secondary">Sign in</a>"#),
        "{html}"
    );
    assert_eq!(
        count(
            &html,
            r#"<a href="/upgrade" class="public-nav-link" aria-current="page">Pricing</a>"#
        ),
        2,
        "{html}"
    );
    assert!(!html.contains("plan-chip"), "{html}");
    assert!(!html.contains(r#"aria-label="Dashboard""#), "{html}");
    assert!(html.contains(r#"<main id="main" class="public-main" tabindex="-1"><div class="page"><div class="page-header"><div><h1>Plans</h1><p class="page-lead">Paid tiers are cost-recovery: they unlock the features that cost real money to run. Everything else stays free.</p></div></div><div class="flash-region" role="status"></div><ul class="pro-features">"#), "{html}");
    assert!(html.contains(
        "<strong>Lower greeting cooldowns</strong><span>Drop the per-member and server-wide limits on <code>/good</code> - for servers busy enough to hit them.</span>"
    ), "{html}");
    assert!(html.contains(&format!(
        r#"<div class="plan-ladder"><div class="plan-card plan-pro"><div class="plan-head"><span class="tier-badge tier-pro">Pro</span><span class="plan-price">$2.99<small>/mo</small></span></div><ul class="plan-specs"><li><strong>50 MB</strong> Palworld save uploads</li><li><strong>30 min</strong> upload cooldown</li></ul><a href="{UPGRADE_URL}" class="btn btn-primary" rel="external noopener noreferrer" target="_blank">Get Pro on upgrade.example</a></div></div>{SUBSCRIBE}"#
    )), "{html}");
    assert!(html.contains(&format!(
        r#"<section class="card" id="kofi" aria-labelledby="kofi-title"><h2 class="label" id="kofi-title">Link your Ko-fi email</h2><p class="page-lead">Connect the email you subscribe with on Ko-fi so your paid membership follows your Discord account.</p><div class="flash-region" role="status"></div><form method="post" action="/upgrade" data-pending="">{}"#,
        kofi_form(None, None)
    )), "{html}");

    app.app
        .entitlements
        .grant(EntitlementScope::User(44), Tier::Pro, "test", "pro-44", None)
        .await
        .unwrap();
    assert_eq!(app.app.entitlements.user_tier(44).await, Tier::Pro);
    let html = app.page("/upgrade", Some(PRO)).await.unwrap();
    assert!(
        html.contains(
            r#"<a href="/guilds" class="btn btn-primary">Open dashboard</a>"#
        ),
        "{html}"
    );
    assert!(!html.contains("plan-chip"), "{html}");
    assert!(
        html.contains(r#"</ul><span class="plan-current">Your plan</span></div>"#),
        "{html}"
    );
    assert!(!html.contains("Get Pro"), "{html}");

    let response =
        app.post("/upgrade", "email=fan%40example.com", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));

    let response =
        app.post("/upgrade", "email=Fan%40Example.com", Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/upgrade#kofi"));
    let flash = flash_cookie(&response).unwrap();
    let html =
        app.page("/upgrade", Some(&format!("{MEMBER}; {flash}"))).await.unwrap();
    assert!(html.contains(&format!(
        r#"<div class="flash-region" role="status"><p class="flash flash-success" data-flash="">{ICON_OPEN}{CHECK}<span class="flash-text">Ko-fi email linked. Your paid membership now follows your Discord account.</span></p></div><form method="post" action="/upgrade" data-pending="">{}"#,
        kofi_form(None, None)
    )), "{html}");
    assert!(html.contains("<title>Plans - Zayden</title>"), "{html}");

    let response =
        app.post("/upgrade", "email=fan%40example.com", Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    let taken = "This Ko-fi email is already linked to an account.";
    assert!(html.contains(&format!(
        r#"<form method="post" action="/upgrade" data-pending=""><div class="error" id="kofi-summary" role="alert" tabindex="-1" autofocus="">Not linked: {taken}</div>{}"#,
        kofi_form(Some("fan@example.com"), Some(taken))
    )), "{html}");

    let response = app.post("/upgrade", "email=nope", Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    let invalid = "Enter the email address you use on Ko-fi.";
    assert!(html.contains(&format!(
        r#"<div class="error" id="kofi-summary" role="alert" tabindex="-1" autofocus="">Not linked: {invalid}</div>{}"#,
        kofi_form(Some("nope"), Some(invalid))
    )), "{html}");

    let unlinked = harness(&pool, None).await.unwrap();
    let html = unlinked.page("/upgrade", None).await.unwrap();
    assert!(
        html.contains(r#"<ul class="plan-specs"><li><strong>50 MB</strong> Palworld save uploads</li><li><strong>30 min</strong> upload cooldown</li></ul></div></div>"#),
        "{html}"
    );
    assert!(html.contains(SUBSCRIBE), "{html}");
    assert!(!html.contains("Get Pro"), "{html}");
    let html = unlinked.page("/guilds", Some(MEMBER)).await.unwrap();
    assert!(html.contains(FREE_CHIP), "{html}");

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
    assert!(!html.contains(r#"class="btn btn-secondary">Sign in</a>"#), "{html}");
    assert!(!html.contains("Open dashboard"), "{html}");
    assert!(!html.contains("plan-chip"), "{html}");
    assert!(!html.contains("plan-ladder"), "{html}");
    assert!(!html.contains(SUBSCRIBE), "{html}");
    assert!(
        html.contains(
            r#"<h2 class="label" id="kofi-title">Link your Ko-fi email</h2>"#
        ),
        "{html}"
    );

    let html = app.page("/guilds", cookie).await.unwrap();
    assert!(
        html.contains(
            r#"<section class="error-panel"><h1 class="error-title">Couldn't load your servers</h1><p class="error-text">Your server list couldn't be loaded: "#
        ),
        "{html}"
    );
    assert!(
        html.contains(r#"<div class="error-actions"><a href="/guilds" class="btn btn-primary">Try again</a><a href="/invite" rel="external" class="btn btn-secondary">Add Zayden to a server</a></div>"#),
        "{html}"
    );
    assert!(!html.contains("error running server function"), "{html}");
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
