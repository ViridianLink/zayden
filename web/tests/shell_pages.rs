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
use secrecy::SecretString;
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
use web::app::guild::guild_id::GuildId;
use web::auth::{SessionUser, WebRole, has_role};
use web::components::brand::LOGO;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::nav::{GENERAL, MODULES};
use web::state::{DiscordState, SessionIdentity, SessionUsersCache, WebState};
use zayden_app::config::BotConfig;
use zayden_app::entitlement::{EntitlementScope, Tier};
use zayden_app::state::AppState as ZaydenAppState;

mod canonical;
mod support;

use canonical::canonical;

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

#[page("/guild/{guild_id}/probe")]
async fn guild_probe(cx: &Cx) -> ViewResult<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    Ok(view! {
        <p id="probe">
            (guild_id)
            "/probe"
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
        discord_token: SecretString::from("test-bot-token"),
        bungie_api_key: SecretString::from(""),
        ai_provider_key: SecretString::from(""),
        google_api_key: SecretString::from(""),
        discord_client_secret: SecretString::from("test-client-secret"),
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
        deadlock_api_key: None,
        youtube_cookies: None,
        palworld_paldex_url: None,
        palworld_palcalc_url: None,
        palworld_save_dir: None,
        palworld_uploads_dir: PathBuf::new(),
        pelican: None,
        jellyfin: None,
        hosting: None,
        redirect_uri: "http://localhost:3000/auth/callback".to_owned(),
        dashboard_url: "http://localhost:3000".to_owned(),
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
        member_names: state.discord.member_names,
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
    let base = web::router()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .app_context(state)
        .page(guild_probe);
    Ok(base.build())
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
        r#"<nav class="{NAV}" aria-label="Dashboard"><ul class="{NAV_LIST}">{}</ul><ul class="{NAV_PLANS}">{}</ul>{admin}</nav>"#,
        nav_link("/guilds", Some(SERVER), "Servers", servers),
        nav_link("/upgrade", Some(ZAP), "Plans", plans),
    )
}

const SERVER: &str = r#"<rect width="20" height="8" x="2" y="2" rx="2"/><rect width="20" height="8" x="2" y="14" rx="2"/><line x1="6" x2="6.01" y1="6" y2="6"/><line x1="6" x2="6.01" y1="18" y2="18"/></svg>"#;
const ZAP: &str = r#"<path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"/></svg>"#;
const GRID: &str = r#"<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/></svg>"#;
const SETTINGS: &str = r#"<line x1="21" x2="14" y1="4" y2="4"/><line x1="10" x2="3" y1="4" y2="4"/><line x1="21" x2="12" y1="12" y2="12"/><line x1="8" x2="3" y1="12" y2="12"/><line x1="21" x2="16" y1="20" y2="20"/><line x1="12" x2="3" y1="20" y2="20"/><line x1="14" x2="14" y1="2" y2="6"/><line x1="8" x2="8" y1="10" y2="14"/><line x1="16" x2="16" y1="18" y2="22"/></svg>"#;
const MAIN: &str = r#"<main id="main" class="app-main" tabindex="-1">"#;
const PLAN_CHIP: &str = "max-md:hidden hover:bg-foreground/5";
const FREE_CHIP: &str = r#"<a href="/upgrade" class="inline-flex w-fit shrink-0 items-center justify-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium whitespace-nowrap border-border text-foreground max-md:hidden hover:bg-foreground/5" aria-label="Upgrade, current plan: Free">Upgrade</a>"#;
const TOPBAR: &str = "fixed top-0 left-0 right-0 z-100 flex items-center gap-3 h-(--topbar-h) py-0 px-4 bg-bg border-b border-solid border-b-rule max-md:gap-2 max-md:px-3";
const BRAND: &str = r#"<a href="/guilds" class="brand"><img class="flex-none w-[28px] h-[28px] rounded-(--radius-full)" src="/_topcoat/assets/logo-0123456789abcdef.png" alt="" width="28" height="28"><span class="max-md:hidden">Zayden</span></a>"#;
const NAV: &str = "flex flex-col gap-1";
const NAV_LIST: &str = "flex flex-col gap-1 m-0 p-0 list-none";
const NAV_PLANS: &str = "flex flex-col gap-1 m-0 mt-4 p-0 list-none";
const NAV_GROUP: &str = "flex flex-col gap-1 mt-4";
const NAV_HEADING: &str = "m-0 py-1 px-3 font-mono text-[12px] font-medium tracking-[0.06em] uppercase text-text-3";
const EMPTY_STATE: &str = "flex flex-col items-start gap-2 max-w-(--form-max) py-8 px-6 border border-dashed border-line rounded-panel";
const ERROR_PANEL: &str = "flex flex-col gap-3 max-w-(--form-max) p-6 border border-solid border-danger rounded-panel bg-surface-1";
const LAMP_WRAP: &str = "inline-flex items-center gap-2 whitespace-nowrap";
const LAMP_DOT: &str =
    "inline-block w-[10px] h-[10px] shrink-0 box-border rounded-(--radius-full)";
const LAMP_OFF: &str = "inline-block w-[10px] h-[10px] shrink-0 box-border rounded-(--radius-full) border-2 border-solid border-line bg-transparent";
const FLASH_TEXT: &str = r#"<span class="text-text">"#;
const FIELD_HELP: &str = "m-0 text-[13px] text-text-2";
const PLATE_PLACEHOLDER: &str = "w-[24px] h-[24px] rounded-control object-cover shrink-0 grid place-items-center bg-surface-2 border border-solid border-line text-text text-[12px] font-semibold";
const PLATE_IMAGE: &str = "w-[24px] h-[24px] rounded-control object-cover shrink-0";
const GUILD_PLACEHOLDER: &str = "grid h-[48px] w-[48px] shrink-0 place-items-center rounded-control border border-solid border-line bg-surface-2 object-cover text-[18px] font-semibold text-text";
const GUILD_IMAGE: &str = "h-[48px] w-[48px] shrink-0 rounded-control object-cover";
const GUILD_GRID: &str =
    "grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4";
const RACK_ROW: &str = "grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-x-4 min-h-(--row-h) py-3 px-4 border-t border-solid border-t-rule first:[border-top:0] max-md:grid-cols-[minmax(0,1fr)_auto] max-md:gap-y-2 max-md:min-h-[48px]";
const RACK_STATE: &str = "min-w-[7rem] max-md:col-[1] max-md:row-[2]";
const RACK_CONTROL: &str = "flex items-center justify-end min-w-[48px] max-md:col-[2] max-md:row-[1/span_2]";
const RACK_LINK: &str =
    "inline-flex items-center gap-1 min-h-(--target-min) text-text font-medium";
const PLAN_SPECS: &str = r#"<ul class="list-none m-0 p-0 flex flex-col gap-2 flex-1"><li class="text-[13.5px] text-text-2"><strong class="font-semibold text-text">50 MB</strong> Palworld save uploads</li><li class="text-[13.5px] text-text-2"><strong class="font-semibold text-text">30 min</strong> upload cooldown</li></ul>"#;
const ERROR_ALERT: &str = "grid w-full grid-cols-[0_1fr] items-start gap-y-1 rounded-lg border bg-background px-4 py-3 text-sm has-[>svg]:grid-cols-[1rem_1fr] has-[>svg]:gap-x-3 [&amp;>svg]:size-4 [&amp;>svg]:translate-y-0.5 border-destructive/50 text-destructive";
const ALERT_TITLE: &str = r#"<p class="col-start-2 font-medium tracking-tight">"#;
const TIER_BADGE: &str = "inline-flex w-fit shrink-0 items-center justify-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium whitespace-nowrap border-transparent bg-foreground/5 text-foreground";
const LEGAL: &str = r#"<nav class="flex flex-wrap gap-y-1 gap-x-4 mt-4 p-3 border-t border-solid border-t-rule text-[13px]" aria-label="Legal"><a href="/privacy" class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]">Privacy Policy</a><a href="/terms" class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]">Terms of Service</a></nav>"#;

const SHIELD: &str = r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/></svg>"#;
const GAMEPAD: &str = r#"<line x1="6" x2="10" y1="11" y2="11"/><line x1="8" x2="8" y1="9" y2="13"/><line x1="15" x2="15.01" y1="12" y2="12"/><line x1="18" x2="18.01" y1="10" y2="10"/><path d="M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z"/></svg>"#;
fn subscribe() -> String {
    format!(
        r#"<div class="flex flex-wrap gap-3 mb-8"><p class="page-lead">Prefer to pay through Discord? Add Zayden to a server first, then subscribe from Zayden's profile in Discord.</p><a href="/invite" rel="external" class="{}">Add Zayden to Discord</a></div>"#,
        support::btn("secondary")
    )
}

fn overview_header() -> String {
    format!(
        r#"<div class="page"><div class="page-header"><div><h1>Overview</h1><p class="page-lead">Turn modules on or off for this server. A module that's off has its commands removed from the server.</p></div><a href="/guild/7/settings" class="{}">Server settings</a></div>"#,
        support::btn("secondary")
    )
}

const NOT_SYNCED: &str = r#"<span class="lamp-sync inline-block w-[10px] h-[10px] shrink-0 box-border rounded-(--radius-full)" aria-hidden="true"></span>"#;

fn row() -> String {
    format!(r#"<li class="{RACK_ROW}">"#)
}

fn lamp_dot(on: bool) -> String {
    if on { format!("lamp-on {LAMP_DOT}") } else { LAMP_OFF.to_owned() }
}

/// The failure alert a rack row shows under its own module.
fn rack_error() -> String {
    format!("{ERROR_ALERT} col-span-full m-0 mt-2")
}

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
    let (text, checked, wanted) =
        if on { ("On", "true", "false") } else { ("Off", "false", "true") };
    let dot = lamp_dot(on);
    format!(
        r#"<form class="contents" method="post" action="/guild/7" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="module_id" value="{module_id}"><div class="{RACK_STATE}"><span class="{LAMP_WRAP}"><span class="{dot}" aria-hidden="true"></span><span class="lamp-text" data-pending-text="Saving…">{text}</span></span></div><div class="{RACK_CONTROL}"><button type="submit" class="switch" role="switch" aria-checked="{checked}" aria-label="{label} module" name="enabled" value="{wanted}"><span class="switch-thumb" aria-hidden="true"></span></button></div></form>"#
    )
}

/// The status and link a module switched on its own page shows in the rack.
fn rack_status(module_path: &str, on: bool) -> String {
    let text = if on { "On" } else { "Off" };
    let dot = lamp_dot(on);
    format!(
        r#"<div class="{RACK_STATE}"><span class="{LAMP_WRAP}"><span class="{dot}" aria-hidden="true"></span><span class="lamp-text">{text}</span></span></div><div class="{RACK_CONTROL}"><a href="/guild/7/{module_path}" class="{RACK_LINK}">Manage{ICON_OPEN}<path d="m9 18 6-6-6-6"/></svg></a></div>"#
    )
}

/// The result line a rack group shows after a switch.
fn group_flash(group: &str, message: &str) -> String {
    format!(
        r#"<div id="modules-{group}"><div class="flash-region" role="status"><p class="flash flash-success" data-flash="">{ICON_OPEN}{CHECK}{FLASH_TEXT}{message}</span></p></div><section class="mb-8""#
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
    assert!(html.contains(&format!(
        r##"<body><a class="skip-link" href="#main">Skip to main content</a><div class="min-h-screen"><header><div class="{TOPBAR}"><button type="button" class="topbar-button menu-button" popovertarget="nav-sheet" aria-controls="nav-sheet" aria-expanded="false" aria-label="Menu">"##
    )), "{html}");
    let sorted = canonical(&html);
    assert!(sorted.contains(&canonical(BRAND)), "{html}");
    assert!(
        html.contains(&format!(
            r#"<a href="/logout" rel="external" class="menu-item">{ICON_OPEN}"#
        )),
        "{html}"
    );
    assert!(sorted.contains(&canonical(FREE_CHIP)), "{html}");
    let nav = outer_nav(true, false, "");
    assert_eq!(count(&sorted, &canonical(&nav)), 2, "{html}");
    assert!(
        sorted.contains(&canonical(&format!(
            r#"<div class="rail">{nav}<div class="flex-1"></div>{LEGAL}</div>"#
        ))),
        "{html}"
    );
    assert!(sorted.contains(&canonical(&format!(
        r#"</header>{MAIN}<div class="page"><div class="page-header"><div><h1>Servers</h1><p class="page-lead">Pick a server to configure Zayden.</p></div><a href="/invite" rel="external" class="{}">Add Zayden to a server</a></div><div class="{GUILD_GRID}">"#,
        support::btn("secondary")
    ))), "{html}");
    assert!(sorted.contains(&canonical(&format!(
        r#"<a href="/guild/7" class="guild-card"><span class="{GUILD_PLACEHOLDER}">G</span><div class="min-w-0"><div class="truncate font-semibold">Guild 7</div>"#
    ))), "{html}");
    assert!(sorted.contains(&canonical(&format!(
        r#"<a href="/guild/9" class="guild-card"><img src="https://cdn.discordapp.com/icons/9/a_0123456789abcdef0123456789abcdef.png?size=64" alt="" class="{GUILD_IMAGE}">"#
    ))), "{html}");
    assert!(!html.contains("Guild 8"), "{html}");
    assert!(html.contains("</div></div></main></div></body>"), "{html}");
    assert!(!html.contains("server-switcher"));

    assert!(has_role(&pool, 42, WebRole::Admin).await.unwrap());
    assert!(has_role(&pool, 42, WebRole::Operator).await.unwrap());
    let html = app.page("/guilds", Some(STAFF)).await.unwrap();
    for prefix in ["rail", "sheet"] {
        let admin = format!(
            r#"<div class="{NAV_GROUP}"><p class="{NAV_HEADING}" id="{prefix}-admin">Admin</p><ul class="{NAV_LIST}" aria-labelledby="{prefix}-admin">{}{}</ul></div>"#,
            nav_link("/admin/servers", Some(SHIELD), "All bot servers", false),
            nav_link(
                "/admin/destiny2/loadouts",
                Some(GAMEPAD),
                "Loadout builder",
                false
            ),
        );
        assert!(
            canonical(&html).contains(&canonical(&outer_nav(true, false, &admin))),
            "{html}"
        );
    }
    assert!(
        canonical(&html).contains(&canonical(&format!(
            r#"<h1>Servers</h1><p class="page-lead">Pick a server to configure Zayden.</p></div></div><div class="{EMPTY_STATE}"><h2 class="m-0 text-[17px] font-semibold">Add Zayden to a server you manage</h2><p class="m-0 mb-2 text-text-2">Servers appear here when you have Manage Server in them.</p><a href="/invite" rel="external" class="{}">Add Zayden to a server</a></div>"#,
            support::btn("primary")
        ))),
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
    let sorted = canonical(&html);
    assert!(sorted.contains(&canonical(BRAND)), "{html}");
    assert!(html.contains(&format!(
        r#"<button type="button" class="topbar-button plate" popovertarget="server-switcher" aria-controls="server-switcher" aria-expanded="false" aria-label="Switch server, current: Guild 7"><span class="{PLATE_PLACEHOLDER}" aria-hidden="true">G</span><span class="plate-name">Guild 7</span>{ICON_OPEN}{CHEVRON_DOWN}</button>"#
    )), "{html}");
    assert!(sorted.contains(&canonical(&format!(
        r#"<div class="menu-list"><a href="/guild/7" class="menu-item" aria-current="page" data-filter-item=""><span class="{PLATE_PLACEHOLDER}" aria-hidden="true">G</span><span class="plate-name">Guild 7</span>{ICON_OPEN}{CHECK}</a><a href="/guild/9" class="menu-item" data-filter-item=""><img src="https://cdn.discordapp.com/icons/9/a_0123456789abcdef0123456789abcdef.png?size=64" alt="" width="24" height="24" class="{PLATE_IMAGE}"><span class="plate-name">Nine</span></a></div><a href="/guilds" class="menu-item">{ICON_OPEN}{SERVER}<span class="plate-name">All servers</span></a>"#
    ))), "{html}");
    assert!(!html.contains("operator-badge"), "{html}");
    assert!(sorted.contains(&canonical(&format!(
        r#"<nav class="{NAV}" aria-label="Dashboard"><ul class="{NAV_LIST}">{}{}</ul><div class="{NAV_GROUP}"><p class="{NAV_HEADING}" id="rail-community">Community</p><ul class="{NAV_LIST}" aria-labelledby="rail-community">{}"#,
        nav_link("/guild/7", Some(GRID), "Overview", true),
        nav_link("/guild/7/settings", Some(SETTINGS), "Server settings", false),
        nav_link("/guild/7/greetings", None, "Greetings", false),
    ))), "{html}");
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
            count(&sorted, &canonical(&nav_link(&href, icon, module.label, false))),
            2,
            "{href}: {html}"
        );
    }
    assert_eq!(
        count(&sorted, &canonical(&nav_link("/upgrade", Some(ZAP), "Plans", false))),
        2,
        "{html}"
    );
    assert!(
        sorted.contains(&canonical(&format!(
            r#"{MAIN}{}<div class="flash-region" role="status"></div><div id="modules-community"><section class="mb-8" aria-labelledby="rack-community"><h2 class="m-0 mb-2 label" id="rack-community">Community</h2><ul class="rack"><li class="{RACK_ROW}"><div class="min-w-0"><h3 class="m-0 text-[15px] font-semibold"><a class="inline-flex items-center min-h-(--target-min) max-md:min-h-(--row-h) text-text no-underline decoration-current hover:underline" href="/guild/7/greetings">Greetings</a></h3>"#,
            overview_header()
        ))),
        "{html}"
    );
    for (group, heading, title) in [
        ("support", "support---safety", "Support &amp; safety"),
        ("voice", "voice---games", "Voice &amp; games"),
        ("integrations", "integrations", "Integrations"),
    ] {
        assert!(
            html.contains(&format!(
                r#"<div id="modules-{group}"><section class="mb-8" aria-labelledby="rack-{heading}"><h2 class="m-0 mb-2 label" id="rack-{heading}">{title}</h2><ul class="rack">"#
            )),
            "{group}: {html}"
        );
    }
    assert_eq!(count(&html, &row()), 16, "{html}");
    assert_eq!(count(&html, NOT_SYNCED), 12, "{html}");
    assert_eq!(count(&html, r#"role="switch""#), 1, "{html}");
    assert!(
        html.contains(&rack_switch("gambling", "Gambling &amp; Economy", true)),
        "{html}"
    );
    for path in ["ai", "patreon", "youtube"] {
        assert!(
            sorted.contains(&canonical(&rack_status(path, false))),
            "{path}: {html}"
        );
    }
    assert!(!html.contains("Toggle module"), "{html}");
    assert!(!html.contains("module-card"), "{html}");

    let html = app.page("/guild/7/probe", Some(MEMBER)).await.unwrap();
    let sorted = canonical(&html);
    assert!(
        sorted.contains(&canonical(&nav_link(
            "/guild/7",
            Some(GRID),
            "Overview",
            false
        ))),
        "{html}"
    );
    assert!(
        html.contains(&format!(r#"{MAIN}<p id="probe">7/probe</p></main>"#)),
        "{html}"
    );
    assert_eq!(
        count(&sorted, r#"aria-current="page" class="nav-link""#),
        0,
        "{html}"
    );

    let html = app.page("/guild/7/settings", Some(MEMBER)).await.unwrap();
    let sorted = canonical(&html);
    let general =
        nav_link("/guild/7/settings", Some(SETTINGS), "Server settings", true);
    assert_eq!(count(&sorted, &canonical(&general)), 2, "{html}");
    assert_eq!(
        count(&sorted, r#"aria-current="page" class="nav-link""#),
        2,
        "{html}"
    );

    let html = app.page("/guild/abc", Some(MEMBER)).await.unwrap();
    let sorted = canonical(&html);
    assert!(sorted.contains(&canonical(&format!(
        r#"{MAIN}<div class="page"><section class="{ERROR_PANEL}"><h1 class="error-title">Server not found</h1><p class="m-0 text-text-2">That address doesn't name a Discord server.</p><div class="flex flex-wrap gap-2"><a href="/guilds" class="{}">Back to servers</a></div></section></div></main>"#,
        support::btn("primary")
    ))), "{html}");
    assert!(!html.contains("server-switcher"), "{html}");
    assert!(
        sorted.contains(&canonical(&nav_link(
            "/guild/abc",
            Some(GRID),
            "Overview",
            true
        ))),
        "{html}"
    );

    let crafted = "/guild/1%2F..%2F..%2Flogout";
    let html = app.page(crafted, Some(MEMBER)).await.unwrap();
    assert!(
        canonical(&html).contains(&canonical(&nav_link(
            crafted,
            Some(GRID),
            "Overview",
            true
        ))),
        "{html}"
    );
    assert!(!html.contains(r#"href="/guild/1/"#), "{html}");

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
    assert_eq!(count(&html, &row()), 16, "{html}");

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
    assert!(
        canonical(&html).contains(&canonical(&rack_status("ai", true))),
        "{html}"
    );
    assert_eq!(count(&html, "data-flash"), 1, "{html}");
    let html = app.page("/guild/7", Some(MEMBER)).await.unwrap();
    assert!(!html.contains("data-flash"), "{html}");

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
    assert!(canonical(&html).contains(&canonical(&format!(
        r#"{}<div class="{error}" role="alert">{ALERT_TITLE}Not changed: Patreon is switched on from its own settings page, not from this toggle.</p></div></li>"#,
        rack_status("patreon", false),
        error = rack_error()
    ))), "{html}");
    assert_eq!(count(&html, &rack_error()), 1, "{html}");

    let response = app
        .post("/guild/7", "guild=8&module_id=ai&enabled=false", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(canonical(&html).contains(&canonical(&format!(
        r#"<div class="{error}" role="alert">{ALERT_TITLE}Not changed: invalid value for `guild`</p></div>"#,
        error = rack_error()
    ))), "{html}");
    assert!(app.app.settings.ai.get(7).await.unwrap().enabled);

    let response = app
        .post("/guild/7", "guild=7&module_id=ai&enabled=false&extra=1", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(canonical(&html).contains(&canonical(&format!(
        r#"<div class="{error}" role="alert">{ALERT_TITLE}Not changed: unknown field `extra`</p></div>"#,
        error = rack_error()
    ))), "{html}");

    for (body, message) in [
        ("guild=7&enabled=true", "missing field `module_id`"),
        ("guild=7&module_id=bogus&enabled=true", "unknown module"),
    ] {
        let response = app.post("/guild/7", body, Some(MEMBER)).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let html = body_text(response).await.unwrap();
        assert!(
            canonical(&html).contains(&canonical(&format!(
                r#"<div class="flash-region" role="status"></div><div class="{ERROR_ALERT} error" role="alert">{ALERT_TITLE}Not changed: {message}</p></div><div id="modules-community">"#
            ))),
            "{html}"
        );
        assert_eq!(count(&html, &rack_error()), 0, "{html}");
    }

    let response = app
        .post("/guild/8", "guild=8&module_id=ai&enabled=true", Some(MEMBER))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(canonical(&html).contains(&canonical(&format!(r#"<section class="{ERROR_PANEL}"><h1 class="error-title">You can't manage this server</h1><p class="m-0 text-text-2">You need Manage Server in this server to change its modules.</p><div class="flex flex-wrap gap-2"><a href="/guilds" class="{}">Back to servers</a></div></section>"#, support::btn("primary")))), "{html}");
    assert!(!html.contains("server-switcher"), "{html}");

    pool.close().await;
}

const INPUT: &str = "h-9 max-md:h-11 w-full min-w-0 rounded-lg border border-input bg-popover px-3 text-sm transition-colors outline-none placeholder:text-muted-foreground file:mr-3 file:h-full file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 aria-invalid:border-destructive aria-invalid:focus-visible:ring-destructive focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50";
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
    let primary = support::btn("primary");
    format!(
        r#"<div class="field-row"{invalid_row}><label class="field-label" for="kofi-email">Ko-fi email</label><div class="flex flex-wrap items-center gap-2 mt-3"><input class="{INPUT} flex-1 [&amp;]:min-w-[220px]" id="kofi-email" type="email" name="email" autocomplete="email" placeholder="you@example.com" required=""{value} aria-describedby="{described}"{invalid}><button type="submit" class="{primary}" data-pending-label="Linking…">Link email</button></div><p class="{FIELD_HELP}" id="kofi-email-help">{KOFI_HELP}</p>{line}</div></form></section>"#
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
    assert!(html.contains(r#"<header class="sticky top-0 z-100 flex items-center justify-center h-(--topbar-h) py-0 px-4 bg-bg border-b border-solid border-b-rule">"#), "{html}");
    assert!(
        html.contains(&format!(
            r#"<a href="/auth/discord" rel="external" class="{}">Sign in</a>"#,
            support::btn("secondary")
        )),
        "{html}"
    );
    assert_eq!(
        count(
            &canonical(&html),
            &canonical(
                r#"<a href="/upgrade" class="public-nav-link" aria-current="page">Pricing</a>"#
            )
        ),
        2,
        "{html}"
    );
    assert!(!html.contains(PLAN_CHIP), "{html}");
    assert!(!html.contains(r#"aria-label="Dashboard""#), "{html}");
    assert!(html.contains(r#"<main id="main" class="public-main" tabindex="-1"><div class="page w-full mx-auto my-0 py-16 px-6"><div class="page-header"><div><h1>Plans</h1><p class="page-lead">Paid tiers are cost-recovery: they unlock the features that cost real money to run. Everything else stays free.</p></div></div><div class="flash-region" role="status"></div><ul class="list-none m-0 mb-8 p-0 grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4">"#), "{html}");
    assert!(html.contains(
        "<strong>Lower greeting cooldowns</strong><span>Drop the per-member and server-wide limits on <code>/good</code> - for servers busy enough to hit them.</span>"
    ), "{html}");
    assert!(html.contains(&format!(
        r#"<div class="plan-ladder"><div class="plan-card plan-pro"><div class="flex items-baseline justify-between gap-3"><span class="{TIER_BADGE} tier-pro">Pro</span><span class="text-[22px] font-bold text-text">$2.99<small class="text-[13px] font-medium text-text-2">/mo</small></span></div>{PLAN_SPECS}<a href="{UPGRADE_URL}" class="{}" rel="external noopener noreferrer" target="_blank">Get Pro on upgrade.example</a></div></div>{}"#,
        support::btn("primary"),
        subscribe()
    )), "{html}");
    assert!(canonical(&html).contains(&canonical(&format!(
        r#"<section class="card" id="kofi" aria-labelledby="kofi-title"><h2 class="label" id="kofi-title">Link your Ko-fi email</h2><p class="page-lead">Connect the email you subscribe with on Ko-fi so your paid membership follows your Discord account.</p><div class="flash-region" role="status"></div><form method="post" action="/upgrade" data-pending="">{}"#,
        kofi_form(None, None)
    ))), "{html}");

    app.app
        .entitlements
        .grant(EntitlementScope::User(44), Tier::Pro, "test", "pro-44", None)
        .await
        .unwrap();
    assert_eq!(app.app.entitlements.user_tier(44).await, Tier::Pro);
    let html = app.page("/upgrade", Some(PRO)).await.unwrap();
    assert!(
        canonical(&html).contains(&canonical(&format!(
            r#"<a href="/guilds" class="{}">Open dashboard</a>"#,
            support::btn("primary")
        ))),
        "{html}"
    );
    assert!(!html.contains(PLAN_CHIP), "{html}");
    assert!(
        html.contains(
            r#"</ul><span class="py-2 px-0 rounded-control text-[13px] font-semibold text-center bg-lamp-wash text-accent">Your plan</span></div>"#
        ),
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
    assert!(canonical(&html).contains(&canonical(&format!(
        r#"<div class="flash-region" role="status"><p class="flash flash-success" data-flash="">{ICON_OPEN}{CHECK}{FLASH_TEXT}Ko-fi email linked. Your paid membership now follows your Discord account.</span></p></div><form method="post" action="/upgrade" data-pending="">{}"#,
        kofi_form(None, None)
    ))), "{html}");
    assert!(html.contains("<title>Plans - Zayden</title>"), "{html}");

    let response =
        app.post("/upgrade", "email=fan%40example.com", Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    let taken = "This Ko-fi email is already linked to an account.";
    assert!(canonical(&html).contains(&canonical(&format!(
        r#"<form method="post" action="/upgrade" data-pending=""><div class="{ERROR_ALERT} error" id="kofi-summary" role="alert" tabindex="-1" autofocus="">{ALERT_TITLE}Not linked: {taken}</p></div>{}"#,
        kofi_form(Some("fan@example.com"), Some(taken))
    ))), "{html}");

    let response = app.post("/upgrade", "email=nope", Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    let invalid = "Enter the email address you use on Ko-fi.";
    assert!(canonical(&html).contains(&canonical(&format!(
        r#"<div class="{ERROR_ALERT} error" id="kofi-summary" role="alert" tabindex="-1" autofocus="">{ALERT_TITLE}Not linked: {invalid}</p></div>{}"#,
        kofi_form(Some("nope"), Some(invalid))
    ))), "{html}");

    let unlinked = harness(&pool, None).await.unwrap();
    let html = unlinked.page("/upgrade", None).await.unwrap();
    assert!(html.contains(&format!("{PLAN_SPECS}</div></div>")), "{html}");
    assert!(html.contains(&subscribe()), "{html}");
    assert!(!html.contains("Get Pro"), "{html}");
    let html = unlinked.page("/guilds", Some(MEMBER)).await.unwrap();
    assert!(canonical(&html).contains(&canonical(FREE_CHIP)), "{html}");

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
    assert!(
        !html.contains(&format!(
            r#"class="{}">Sign in</a>"#,
            support::btn("secondary")
        )),
        "{html}"
    );
    assert!(!html.contains("Open dashboard"), "{html}");
    assert!(!html.contains(PLAN_CHIP), "{html}");
    assert!(!html.contains("plan-ladder"), "{html}");
    assert!(!html.contains(&subscribe()), "{html}");
    assert!(
        html.contains(
            r#"<h2 class="label" id="kofi-title">Link your Ko-fi email</h2>"#
        ),
        "{html}"
    );

    let html = app.page("/guilds", cookie).await.unwrap();
    assert!(
        html.contains(&format!(
            r#"<section class="{ERROR_PANEL}"><h1 class="error-title">Couldn't load your servers</h1><p class="m-0 text-text-2">Your server list couldn't be loaded: "#
        )),
        "{html}"
    );
    assert!(
        canonical(&html).contains(&canonical(&format!(
            r#"<div class="flex flex-wrap gap-2"><a href="/guilds" class="{}">Try again</a><a href="/invite" rel="external" class="{}">Add Zayden to a server</a></div>"#,
            support::btn("primary"),
            support::btn("secondary")
        ))),
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

#[sqlx::test(migrations = "../migrations")]
async fn a_missing_page_inside_a_managed_guild_keeps_the_guild_shell(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool, None).await.unwrap();

    let response = app.get("/guild/7/nope", Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let html = body_text(response).await.unwrap();
    assert!(html.contains("<title>Not found - Zayden</title>"), "{html}");
    assert!(html.contains(r#"popovertarget="server-switcher""#), "{html}");
    assert!(canonical(&html).contains(&canonical(&format!(
        r#"{MAIN}<div class="page"><section class="{ERROR_PANEL}"><h1 class="error-title">Page not found</h1><p class="m-0 text-text-2">There is no page at this address in this server. The link may have a typo, or the page may have moved.</p><div class="flex flex-wrap gap-2"><a href="/guild/7" class="{}">Back to Guild 7</a><a href="/guilds" class="{}">Back to servers</a></div></section></div></main>"#,
        support::btn("primary"),
        support::btn("secondary")
    ))), "{html}");

    for path in ["/guild/8/nope", "/guild/abc/nope", "/nope"] {
        let response = app.get(path, Some(MEMBER)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        let html = body_text(response).await.unwrap();
        assert!(!html.contains("server-switcher"), "{path}: {html}");
        assert!(
            canonical(&html).contains(&canonical(&format!(
                r#"<a href="/guilds" class="{} w-full">Back to servers</a>"#,
                support::btn("primary-lg")
            ))),
            "{path}: {html}"
        );
    }

    let response = app.get("/nope", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let html = body_text(response).await.unwrap();
    assert!(
        canonical(&html).contains(&canonical(&format!(
            r#"<a href="/" class="{} w-full">Back home</a>"#,
            support::btn("primary-lg")
        ))),
        "{html}"
    );

    let response = app.get("/guild/7/nope", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    pool.close().await;
}
