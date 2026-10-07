//! The guild settings page and its Server settings, AI, Family, Honeypot, LFG,
//! Music and Temp voice sections through the app router, against Postgres and a
//! stand-in Discord API.
//!
//! Sessions and Discord guild lists come from seeded caches. The bot's
//! Discord client talks to a local server that answers the channel, role,
//! thread and channel-creation endpoints the sections use and records every
//! request. Stored settings are read directly before the markup is checked,
//! so a database fault fails with its own error.

use std::error::Error;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread;

use http_body_util::BodyExt;
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, StatusCode, header};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use web::auth::SessionUser;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::state::{DiscordState, SessionIdentity, SessionUsersCache, WebState};
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const MEMBER: &str = "session=member-token";
const FORM: &str = "application/x-www-form-urlencoded";

/// Two connections per test pool keep the binary inside `sqlx::test`'s
/// 20-permit master pool (see `tests/shell_pages.rs`).
const TEST_POOL_CONNECTIONS: u32 = 2;

const ICON_OPEN: &str = r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">"#;
const CHEVRON: &str = r#"<span class="select-chevron"><svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></span>"#;
const GRID: &str = r#"<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/></svg>"#;
const USERS: &str = r#"<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>"#;
const SAVE: &str = r#"<div class="form-actions"><button type="submit" class="btn btn-primary" data-pending-label="Saving…">Save</button></div>"#;

const TEXT_OPTIONS: &str = r#"<option value="100"># rules</option><option value="101">📢 news</option><option value="104">💬 help</option>"#;
const ROLE_OPTIONS: &str =
    r#"<option value="200">@Mods</option><option value="201">@Members</option>"#;
const CATEGORY_OPTIONS: &str = r#"<option value="102">▸ Voice</option>"#;
const VOICE_OPTIONS: &str = r#"<option value="103">🔊 lobby</option>"#;

#[derive(Clone, Debug)]
struct Hit {
    method: String,
    path: String,
}

struct Reply {
    method: &'static str,
    path: String,
    status: u16,
    body: String,
}

#[derive(Clone)]
struct Discord {
    addr: String,
    hits: Arc<Mutex<Vec<Hit>>>,
    replies: Arc<Mutex<Vec<Reply>>>,
}

impl Discord {
    fn start() -> TestResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let discord = Self {
            addr: listener.local_addr()?.to_string(),
            hits: Arc::default(),
            replies: Arc::default(),
        };

        let served = discord.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let served = served.clone();
                thread::spawn(move || {
                    let _ = served.serve(stream);
                });
            }
        });

        Ok(discord)
    }

    fn serve(&self, stream: TcpStream) -> std::io::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut stream = stream;

        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
        let mut request = request_line.split_whitespace();
        let method = request.next().unwrap_or_default().to_owned();
        let target = request.next().unwrap_or_default().to_owned();

        let mut length = 0;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value.trim().parse::<usize>().unwrap_or(0);
            }
        }
        let mut body = vec![0_u8; length];
        reader.read_exact(&mut body)?;

        let path = target
            .split('?')
            .next()
            .unwrap_or_default()
            .trim_start_matches("/api/v10")
            .to_owned();
        let (status, reply) = self.answer(&method, &path);
        locked(&self.hits).push(Hit { method, path });

        let response = format!(
            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n\
             content-length: {}\r\nconnection: close\r\n\r\n{reply}",
            reply.len()
        );
        stream.write_all(response.as_bytes())
    }

    fn answer(&self, method: &str, path: &str) -> (u16, String) {
        locked(&self.replies)
            .iter()
            .rev()
            .find(|r| r.method == method && r.path == path)
            .map_or_else(
                || (404, r#"{"message":"Unknown","code":0}"#.to_owned()),
                |r| (r.status, r.body.clone()),
            )
    }

    fn reply(&self, method: &'static str, path: &str, status: u16, body: &str) {
        locked(&self.replies).push(Reply {
            method,
            path: path.to_owned(),
            status,
            body: body.to_owned(),
        });
    }

    fn count(&self, method: &str, path: &str) -> usize {
        locked(&self.hits)
            .iter()
            .filter(|h| h.method == method && h.path == path)
            .count()
    }
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn channel_json(id: u64, name: &str, kind: u8, position: u32) -> String {
    format!(
        r#"{{"id":"{id}","type":{kind},"name":"{name}","position":{position},"guild_id":"7"}}"#
    )
}

fn role_json(id: u64, name: &str, position: i64) -> String {
    format!(
        r#"{{"color":0,"colors":{{"primary_color":0}},"hoist":false,"id":"{id}","managed":false,"mentionable":false,"name":"{name}","permissions":"0","position":{position},"flags":0}}"#
    )
}

fn list(items: &[String]) -> String {
    format!("[{}]", items.join(","))
}

/// Guild 7: text, announcement, category, voice and forum channels and two
/// roles besides @everyone. Guild 9 lists its channels but fails its roles.
fn serve_guilds(discord: &Discord) {
    let channels = list(&[
        channel_json(103, "lobby", 2, 3),
        channel_json(100, "rules", 0, 0),
        channel_json(102, "Voice", 4, 2),
        channel_json(101, "news", 5, 1),
        channel_json(104, "help", 15, 4),
    ]);
    let roles = list(&[
        role_json(7, "@everyone", 0),
        role_json(201, "Members", 1),
        role_json(200, "Mods", 2),
    ]);
    discord.reply("GET", "/guilds/7/channels", 200, &channels);
    discord.reply("GET", "/guilds/7/roles", 200, &roles);
    discord.reply("GET", "/guilds/9/channels", 200, &channels);
    discord.reply("GET", "/guilds/9/roles", 500, r#"{"message":"boom","code":0}"#);
    discord.reply(
        "GET",
        "/guilds/10/channels",
        500,
        r#"{"message":"boom","code":0}"#,
    );
    discord.reply("GET", "/guilds/10/roles", 200, &roles);
    discord.reply(
        "GET",
        "/channels/300",
        200,
        r#"{"id":"300","type":11,"name":"raids","guild_id":"7","parent_id":"104"}"#,
    );
    discord.reply(
        "GET",
        "/channels/301",
        200,
        r#"{"id":"301","type":11,"name":"elsewhere","guild_id":"8","parent_id":"1"}"#,
    );
    discord.reply(
        "POST",
        "/guilds/7/channels",
        201,
        r#"{"id":"400","type":2,"name":"➕ Creator Channel","position":5,"guild_id":"7","parent_id":"102"}"#,
    );
}

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-settings-a-assets-{}", std::process::id()));
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
        invite_url: None,
        upgrade_url: None,
        kofi_verification_token: None,
        patreon: None,
        youtube: None,
        discord_sku_pro: None,
        discord_sku_ultra: None,
        radio_stations: Arc::from(Vec::new()),
    }
}

fn guild(id: u64, permissions: Permissions) -> CurrentUserGuild {
    CurrentUserGuild {
        id: Id::new(id),
        name: format!("Guild {id}"),
        icon: None,
        owner: false,
        permissions,
        features: Vec::new(),
    }
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
    discord: Discord,
}

async fn harness(pool: &PgPool) -> TestResult<Harness> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let discord = Discord::start()?;
    serve_guilds(&discord);
    let config = config();
    let app = Arc::new(ZaydenAppState::new(pool.clone(), &config));
    let mut state = WebState::new(Arc::clone(&app), &config)?;
    state.discord = DiscordState {
        http: Arc::new(
            twilight_http::Client::builder()
                .token("test-bot-token".to_owned())
                .proxy(discord.addr.clone(), true)
                .ratelimiter(None)
                .build(),
        ),
        user_guilds: state.discord.user_guilds,
        users: state.discord.users,
    };
    state
        .sessions
        .insert("member-token".to_owned(), SessionIdentity {
            user_id: 41,
            access_token: "member-token-access".to_owned(),
        })
        .await;
    seed_users(&state.discord.users, &[41]).await;
    state
        .discord
        .user_guilds
        .insert(
            41,
            Arc::from([
                guild(7, Permissions::MANAGE_GUILD),
                guild(8, Permissions::SEND_MESSAGES),
                guild(9, Permissions::ADMINISTRATOR),
                guild(10, Permissions::ADMINISTRATOR),
            ]),
        )
        .await;

    let base = Router::builder()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .app_context(state);
    Ok(Harness { router: web::router(base), app, discord })
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

    async fn page(&self, path: &str) -> TestResult<String> {
        let response = self.get(path, Some(MEMBER)).await?;
        if response.status() != StatusCode::OK {
            return Err(format!("{path} answered {}", response.status()).into());
        }
        body_text(response).await
    }

    /// Posts `body` and returns the re-rendered page, checking its status.
    async fn submit(
        &self,
        path: &str,
        body: &str,
        status: StatusCode,
    ) -> TestResult<String> {
        let response = self.post(path, body, Some(MEMBER)).await?;
        if response.status() != status {
            let got = response.status();
            let html = body_text(response).await?;
            return Err(format!("{path} {body} answered {got}: {html}").into());
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

fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

fn location(response: &Response) -> Option<&str> {
    response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok())
}

/// The page's `main`, from the page header to the end of the page div.
fn main_content(html: &str) -> &str {
    html.split_once(r#"<main id="main" class="app-main" tabindex="-1">"#)
        .and_then(|(_, rest)| rest.split_once("</main>"))
        .map_or("", |(main, _)| main)
}

fn header(label: &str, lead: &str) -> String {
    format!(
        r#"<div class="page"><div class="page-header"><div><h1>{label}</h1><p class="page-lead">{lead}</p></div></div>"#
    )
}

fn form(slug: &str, guild: &str) -> String {
    format!(
        r#"<form method="post" action="/guild/{guild}/settings/{slug}" data-pending=""><input type="hidden" name="guild" value="{guild}">"#
    )
}

fn select(label: &str, name: &str, selected: &str, options: &str) -> String {
    let none = if selected.is_empty() { r#" selected="""# } else { "" };
    let options = if selected.is_empty() {
        options.to_owned()
    } else {
        options.replace(
            &format!(r#"<option value="{selected}">"#),
            &format!(r#"<option value="{selected}" selected="">"#),
        )
    };
    format!(
        r#"<div class="setting-field"><label for="field-{name}">{label}</label><div class="select"><select class="input" id="field-{name}" name="{name}"><option value=""{none}>(not set)</option>{options}</select>{CHEVRON}</div></div>"#
    )
}

fn toggle(label: &str, name: &str, on: bool) -> String {
    let (yes, no) =
        if on { (r#" selected="""#, "") } else { ("", r#" selected="""#) };
    format!(
        r#"<div class="setting-field"><label for="field-{name}">{label}</label><div class="select"><select class="input" id="field-{name}" name="{name}"><option value="true"{yes}>Enabled</option><option value="false"{no}>Disabled</option></select>{CHEVRON}</div></div>"#
    )
}

fn text(label: &str, name: &str, value: &str) -> String {
    format!(
        r#"<div class="setting-field"><label for="field-{name}">{label}</label><input class="input" id="field-{name}" type="text" name="{name}" value="{value}" placeholder="(not set)" pattern="[0-9]*"></div>"#
    )
}

fn alert(class: &str, role: &str, message: &str) -> String {
    format!(
        r#"<div class="alert {class}" role="{role}"><span>{message}</span><button type="button" class="alert-dismiss" aria-label="Dismiss">{ICON_OPEN}"#
    )
}

fn saved() -> String {
    alert("success", "status", "Saved.")
}

fn not_saved(message: &str) -> String {
    alert(
        "error",
        "alert",
        &format!("Failed to save: error running server function: {message}"),
    )
}

#[sqlx::test(migrations = "../migrations")]
async fn every_section_renders_its_stored_settings(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let settings = &app.app.settings;

    settings
        .channels
        .update(7, |p| {
            p.rules_channel_id = Some(100);
            p.spoiler_channel_id = Some(999);
        })
        .await
        .unwrap();
    settings.roles.update(7, |p| p.sleep_role_id = Some(201)).await.unwrap();
    let stored = settings.channels.get(7).await.unwrap();
    assert_eq!(stored.rules_channel_id, Some(100));
    assert_eq!(stored.spoiler_channel_id, Some(999));
    assert_eq!(settings.roles.get(7).await.unwrap().sleep_role_id, Some(201));

    let general = format!(
        r#"{}<fieldset class="settings-section"><legend>{ICON_OPEN}{GRID}Channels</legend>{}{}{}{}{SAVE}</form></fieldset><fieldset class="settings-section"><legend>{ICON_OPEN}{USERS}Roles</legend>{}{}{}{}{SAVE}</form></fieldset></div>"#,
        header(
            "Server settings",
            "Server-wide channels and roles the rest of Zayden points at."
        ),
        form("general", "7"),
        select("Rules Channel", "rules_channel_id", "100", TEXT_OPTIONS),
        select("General Channel", "general_channel_id", "", TEXT_OPTIONS),
        select(
            "Spoiler Channel",
            "spoiler_channel_id",
            "999",
            &format!(r#"<option value="999">Unknown (999)</option>{TEXT_OPTIONS}"#)
        ),
        form("general", "7"),
        select("Artist Role", "artist_role_id", "", ROLE_OPTIONS),
        select("Sleep Role", "sleep_role_id", "201", ROLE_OPTIONS),
        select("Verified Role", "verified_role_id", "", ROLE_OPTIONS),
    );
    for path in [
        "/guild/7/settings",
        "/guild/7/settings/general",
        "/guild/7/settings/bogus",
        "/guild/7/settings/greetings",
    ] {
        let html = app.page(path).await.unwrap();
        assert!(
            html.contains("<title>Server settings - Zayden Dashboard</title>"),
            "{path}: {html}"
        );
        assert_eq!(main_content(&html), general, "{path}");
        let active = count(
            &html,
            r#"<a href="/guild/7/settings/general" class="nav-link" aria-current="page">"#,
        );
        let expected = 2 * usize::from(
            !path.ends_with("bogus") && !path.ends_with("greetings"),
        );
        assert_eq!(active, expected, "{path}: {html}");
        assert_eq!(
            count(&html, r#"class="nav-link" aria-current="page""#),
            expected,
            "{path}: {html}"
        );
    }

    settings
        .ai
        .update(7, |p| {
            p.enabled = true;
            p.channel_id = Some(101);
        })
        .await
        .unwrap();
    assert!(settings.ai.get(7).await.unwrap().enabled);
    let html = app.page("/guild/7/settings/ai").await.unwrap();
    assert!(
        html.contains("<title>AI Chat settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert_eq!(
        count(
            &html,
            r#"<a href="/guild/7/settings/ai" class="nav-link" aria-current="page"><span>AI Chat</span></a>"#,
        ),
        2,
        "{html}"
    );
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<fieldset class="settings-section">{}{}{}{SAVE}</form><p class="page-lead">With AI responses on, Zayden replies in character whenever someone mentions him. Leave the channel unset to let him answer anywhere he can see, or pick one to keep him to a single room.</p><p class="page-lead">Every reply costs a model call, so scope this to a channel you actually want him talking in. The toggle here is the same switch as the AI Chat card on the Modules page.</p></fieldset></div>"#,
            header("AI Chat", "Whether Zayden answers when mentioned, and where."),
            form("ai", "7"),
            toggle("AI Responses", "enabled", true),
            select("Restrict to Channel", "channel_id", "101", TEXT_OPTIONS),
        )
    );

    assert_eq!(settings.family.get(7).await.unwrap().max_partners, 1);
    let html = app.page("/guild/7/settings/family").await.unwrap();
    assert!(
        html.contains("<title>Family settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<fieldset class="settings-section">{}{}{SAVE}</form></fieldset></div>"#,
            header("Family", "Limits for the family and relationship commands."),
            form("family", "7"),
            text("Max Partners", "max_partners", "1"),
        )
    );

    let honeypot = settings.honeypot.get(7).await.unwrap();
    assert_eq!(honeypot.purge_seconds, 86_400);
    assert!(!honeypot.exempt_admins);
    let html = app.page("/guild/7/settings/honeypot").await.unwrap();
    assert!(
        html.contains("<title>Honeypot settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<fieldset class="settings-section">{}{}{}{}{}{SAVE}</form><p class="page-lead">Anyone who posts in the honeypot channel is banned - which purges their recent messages server-wide - and then immediately unbanned, so a recovered account can rejoin. Leave the channel unset to turn the trap off.</p><p class="page-lead">The purge window is how far back the ban deletes the offender's messages, across every channel. Defaults to 86400 (24 hours); 0 keeps their history and Discord caps it at 604800 (7 days).</p><p class="page-lead">The server owner is always exempt. Keep the channel postable by @everyone - the trap only catches spam bots that can actually reach it.</p></fieldset></div>"#,
            header(
                "Honeypot",
                "The spam trap: a bait channel that bans whoever posts in it."
            ),
            form("honeypot", "7"),
            select("Honeypot Channel", "channel_id", "", TEXT_OPTIONS),
            toggle("Exempt Admins", "exempt_admins", false),
            select("Exempt Role", "exempt_role_id", "", ROLE_OPTIONS),
            text("Purge Window (seconds)", "purge_seconds", "86400"),
        )
    );

    settings
        .lfg
        .update(7, |p| {
            p.lfg_role_id = Some(200);
            p.lfg_scheduled_thread_id = Some(300);
        })
        .await
        .unwrap();
    assert_eq!(
        settings.lfg.get(7).await.unwrap().lfg_scheduled_thread_id,
        Some(300)
    );
    let html = app.page("/guild/7/settings/lfg").await.unwrap();
    assert!(
        html.contains("<title>LFG settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<fieldset class="settings-section">{}{}{}{}{SAVE}</form></fieldset></div>"#,
            header("LFG", "Where looking-for-group posts go and who they ping."),
            form("lfg", "7"),
            select("LFG Channel", "lfg_channel_id", "", TEXT_OPTIONS),
            select("LFG Role", "lfg_role_id", "200", ROLE_OPTIONS),
            text("LFG Scheduled Thread ID", "lfg_scheduled_thread_id", "300"),
        )
    );

    let music = settings.music.get(7).await.unwrap();
    assert_eq!(music.auto_disconnect_secs, 120);
    assert!(music.announce_now_playing);
    let html = app.page("/guild/7/settings/music").await.unwrap();
    assert!(
        html.contains("<title>Music settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<fieldset class="settings-section">{}{}{}{}{}{SAVE}</form><p class="page-lead">Announcements post when a track ends and the next one starts. Leave the announce channel unset to use the channel /play was run in.</p><p class="page-lead">Default volume, 24/7 mode and autoplay change while music is playing - set those in Discord with /music settings.</p></fieldset></div>"#,
            header("Music", "Playback permissions and now-playing announcements."),
            form("music", "7"),
            select("DJ Role", "dj_role_id", "", ROLE_OPTIONS),
            text("Auto-disconnect (seconds)", "auto_disconnect_secs", "120"),
            toggle("Announce Now Playing", "announce_now_playing", true),
            select("Announce Channel", "announce_channel_id", "", TEXT_OPTIONS),
        )
    );

    settings
        .temp_voice
        .update(7, |p| p.temp_voice_category = Some(102))
        .await
        .unwrap();
    assert_eq!(
        settings.temp_voice.get(7).await.unwrap().temp_voice_category,
        Some(102)
    );
    let html = app.page("/guild/7/settings/temp-voice").await.unwrap();
    assert!(
        html.contains("<title>Temp voice settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<fieldset class="settings-section">{}{}{}{SAVE}</form><p class="page-lead">No creator channel yet? Zayden can make one for you and point the settings above at it.</p>{}{}<div class="form-actions"><button type="submit" class="btn btn-secondary">Create Creator Channel</button></div></form></fieldset></div>"#,
            header(
                "Temp voice",
                "On-demand voice channels created from a join-to-create channel."
            ),
            form("temp-voice", "7"),
            select("Category", "temp_voice_category", "102", CATEGORY_OPTIONS),
            select(
                "Creator Channel",
                "temp_voice_creator_channel",
                "",
                VOICE_OPTIONS
            ),
            form("temp-voice", "7"),
            select(
                "Create Creator Channel In",
                "temp_voice_category",
                "102",
                CATEGORY_OPTIONS
            )
            .replace("field-temp_voice_category", "temp-voice-create-category"),
        )
    );

    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 0);
    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn saves_re_render_with_inline_feedback(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let settings = &app.app.settings;

    let html = app
        .submit(
            "/guild/7/settings/general",
            "guild=7&rules_channel_id=101&general_channel_id=&spoiler_channel_id=104",
            StatusCode::OK,
        )
        .await
        .unwrap();
    let stored = settings.channels.get(7).await.unwrap();
    assert_eq!(stored.rules_channel_id, Some(101));
    assert_eq!(stored.general_channel_id, None);
    assert_eq!(stored.spoiler_channel_id, Some(104));
    assert!(
        html.contains("<title>Server settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            "Channels</legend>{}{}{}",
            saved(),
            r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg></button></div>"#,
            form("general", "7"),
        )),
        "{html}"
    );
    assert!(
        html.contains(&select(
            "Rules Channel",
            "rules_channel_id",
            "101",
            TEXT_OPTIONS
        )),
        "{html}"
    );
    assert!(
        html.contains(&format!("Roles</legend>{}", form("general", "7"))),
        "{html}"
    );
    assert_eq!(count(&html, "alert success"), 1, "{html}");

    let html = app
        .submit(
            "/guild/7/settings/general",
            "guild=7&artist_role_id=999&sleep_role_id=200&verified_role_id=",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(settings.roles.get(7).await.unwrap().sleep_role_id, None);
    assert!(
        html.contains(&format!(
            "Roles</legend>{}",
            not_saved("that role is not in this server")
        )),
        "{html}"
    );
    assert!(
        html.contains(&select(
            "Artist Role",
            "artist_role_id",
            "999",
            &format!(r#"<option value="999">Unknown (999)</option>{ROLE_OPTIONS}"#)
        )),
        "{html}"
    );
    assert!(
        html.contains(&select("Sleep Role", "sleep_role_id", "200", ROLE_OPTIONS)),
        "{html}"
    );
    assert!(
        html.contains(&select(
            "Rules Channel",
            "rules_channel_id",
            "101",
            TEXT_OPTIONS
        )),
        "{html}"
    );
    assert_eq!(count(&html, "alert "), 1, "{html}");

    for (body, message) in [
        (
            "guild=8&rules_channel_id=&general_channel_id=&spoiler_channel_id=",
            "invalid value for `guild`",
        ),
        (
            "guild=7&rules_channel_id=&general_channel_id=&spoiler_channel_id=&x=1",
            "unknown field `x`",
        ),
        ("guild=7&rules_channel_id=", "missing field `general_channel_id`"),
        ("guild=7", "missing field `artist_role_id`"),
        (
            "guild=7&rules_channel_id=&general_channel_id=&spoiler_channel_id=&artist_role_id=",
            "unknown field `artist_role_id`",
        ),
    ] {
        let html = app
            .submit(
                "/guild/7/settings/general",
                body,
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await
            .unwrap();
        assert!(html.contains(&not_saved(message)), "{body}: {html}");
    }
    assert_eq!(settings.channels.get(7).await.unwrap().rules_channel_id, Some(101));

    let html = app
        .submit(
            "/guild/7/settings/ai",
            "guild=7&enabled=true&channel_id=100",
            StatusCode::OK,
        )
        .await
        .unwrap();
    let ai = settings.ai.get(7).await.unwrap();
    assert!(ai.enabled);
    assert_eq!(ai.channel_id, Some(100));
    assert!(
        html.contains(&format!(r#"<fieldset class="settings-section">{}"#, saved())),
        "{html}"
    );
    assert!(html.contains(&toggle("AI Responses", "enabled", true)), "{html}");

    let html = app
        .submit("/guild/7/settings/family", "guild=7&max_partners=0", StatusCode::OK)
        .await
        .unwrap();
    assert_eq!(settings.family.get(7).await.unwrap().max_partners, 1);
    assert!(html.contains(&saved()), "{html}");
    assert!(html.contains(&text("Max Partners", "max_partners", "0")), "{html}");

    let html = app
        .submit(
            "/guild/7/settings/honeypot",
            "guild=7&channel_id=100&exempt_admins=true&exempt_role_id=&purge_seconds=999999",
            StatusCode::OK,
        )
        .await
        .unwrap();
    let honeypot = settings.honeypot.get(7).await.unwrap();
    assert_eq!(honeypot.channel_id, Some(100));
    assert!(honeypot.exempt_admins);
    assert_eq!(honeypot.purge_seconds, 604_800);
    assert!(html.contains(&saved()), "{html}");
    assert!(
        html.contains(&text("Purge Window (seconds)", "purge_seconds", "999999")),
        "{html}"
    );
    let html = app
        .submit(
            "/guild/7/settings/honeypot",
            "guild=7&channel_id=abc&exempt_admins=false&exempt_role_id=&purge_seconds=",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(
        html.contains(&not_saved(
            "`abc` is not a valid channel id. Leave the field blank to clear it."
        )),
        "{html}"
    );
    assert!(settings.honeypot.get(7).await.unwrap().exempt_admins);

    let html = app
        .submit(
            "/guild/7/settings/lfg",
            "guild=7&lfg_channel_id=104&lfg_role_id=201&lfg_scheduled_thread_id=300",
            StatusCode::OK,
        )
        .await
        .unwrap();
    assert_eq!(
        settings.lfg.get(7).await.unwrap().lfg_scheduled_thread_id,
        Some(300)
    );
    assert!(html.contains(&saved()), "{html}");
    let html = app
        .submit(
            "/guild/7/settings/lfg",
            "guild=7&lfg_channel_id=&lfg_role_id=&lfg_scheduled_thread_id=301",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(
        html.contains(&not_saved("that channel is not in this server")),
        "{html}"
    );
    assert!(
        html.contains(&text(
            "LFG Scheduled Thread ID",
            "lfg_scheduled_thread_id",
            "301"
        )),
        "{html}"
    );
    assert_eq!(settings.lfg.get(7).await.unwrap().lfg_role_id, Some(201));

    let html = app
        .submit(
            "/guild/7/settings/music",
            "guild=7&dj_role_id=200&auto_disconnect_secs=60&announce_now_playing=false&announce_channel_id=101",
            StatusCode::OK,
        )
        .await
        .unwrap();
    let music = settings.music.get(7).await.unwrap();
    assert_eq!((music.dj_role_id, music.auto_disconnect_secs), (Some(200), 60));
    assert!(!music.announce_now_playing);
    assert!(html.contains(&saved()), "{html}");
    assert!(
        html.contains(&toggle(
            "Announce Now Playing",
            "announce_now_playing",
            false
        )),
        "{html}"
    );

    let html = app
        .submit(
            "/guild/7/settings/temp-voice",
            "guild=7&temp_voice_category=102&temp_voice_creator_channel=103",
            StatusCode::OK,
        )
        .await
        .unwrap();
    let temp_voice = settings.temp_voice.get(7).await.unwrap();
    assert_eq!(temp_voice.temp_voice_creator_channel, Some(103));
    assert!(
        html.contains(&format!(r#"<fieldset class="settings-section">{}"#, saved())),
        "{html}"
    );
    assert_eq!(count(&html, "alert "), 1, "{html}");

    let html = app
        .submit(
            "/guild/7/settings/temp-voice",
            "guild=7&temp_voice_category=",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(html.contains(&format!(
        "it.</p>{}",
        alert(
            "error",
            "alert",
            "Failed to create channel: error running server function: select a category first"
        )
    )), "{html}");
    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 0);

    for (body, message) in [
        ("guild=8&temp_voice_category=102", "invalid value for `guild`"),
        (
            "guild=7&temp_voice_category=102&artist_role_id=",
            "unknown field `artist_role_id`",
        ),
    ] {
        let html = app
            .submit(
                "/guild/7/settings/temp-voice",
                body,
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await
            .unwrap();
        assert!(
            html.contains(&alert(
                "error",
                "alert",
                &format!("Failed to create channel: error running server function: {message}")
            )),
            "{body}: {html}"
        );
    }
    let html = app
        .submit(
            "/guild/7/settings/temp-voice",
            "guild=7&temp_voice_category=102&temp_voice_creator_channel=103&x=1",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(
        html.contains(&format!(
            r#"<fieldset class="settings-section">{}"#,
            not_saved("unknown field `x`")
        )),
        "{html}"
    );
    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 0);

    let response = app
        .post(
            "/guild/7/settings/temp-voice",
            "guild=7&temp_voice_category=102",
            Some(MEMBER),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/guild/7/settings/temp-voice?created=1"));
    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 1);
    let temp_voice = settings.temp_voice.get(7).await.unwrap();
    assert_eq!(temp_voice.temp_voice_category, Some(102));
    assert_eq!(temp_voice.temp_voice_creator_channel, Some(400));

    app.discord.reply(
        "GET",
        "/guilds/7/channels",
        200,
        &list(&[
            channel_json(100, "rules", 0, 0),
            channel_json(102, "Voice", 4, 2),
            channel_json(103, "lobby", 2, 3),
            channel_json(400, "\u{2795} Creator Channel", 2, 5),
        ]),
    );
    let html = app.page("/guild/7/settings/temp-voice?created=1").await.unwrap();
    assert!(
        html.contains("<title>Temp voice settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            "it.</p>{}",
            alert("success", "status", "Creator channel created.")
        )),
        "{html}"
    );
    assert!(
        html.contains(&select(
            "Creator Channel",
            "temp_voice_creator_channel",
            "400",
            &format!(r#"{VOICE_OPTIONS}<option value="400">🔊 ➕ Creator Channel</option>"#)
        )),
        "{html}"
    );
    assert_eq!(count(&html, "alert "), 1, "{html}");
    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 1);

    for path in [
        "/guild/7/settings/temp-voice?created=0",
        "/guild/7/settings/temp-voice?created=yes",
        "/guild/7/settings/temp-voice?created",
    ] {
        let html = app.page(path).await.unwrap();
        assert_eq!(count(&html, "alert "), 0, "{path}: {html}");
    }

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn settings_report_what_they_cannot_load(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();

    for path in
        ["/guild/7/settings", "/guild/7/settings/ai", "/guild/7/settings/bogus"]
    {
        let response = app.get(path, None).await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(location(&response), Some("/login"), "{path}");
    }
    let response = app
        .post("/guild/7/settings/family", "guild=7&max_partners=3", None)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));
    assert_eq!(app.app.settings.family.get(7).await.unwrap().max_partners, 1);

    let html = app.page("/guild/8/settings/music").await.unwrap();
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<p class="error">Failed to load settings: error running server function: forbidden</p></div>"#,
            header("Music", "Playback permissions and now-playing announcements."),
        )
    );
    let html = app
        .submit(
            "/guild/8/settings/family",
            "guild=8&max_partners=3",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}{}<path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg></button></div><p class="error">Failed to load settings: error running server function: forbidden</p></div>"#,
            header("Family", "Limits for the family and relationship commands."),
            not_saved("forbidden"),
        )
    );
    let html = app
        .submit(
            "/guild/8/settings/temp-voice",
            "guild=8&temp_voice_category=102",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<p class="error">Failed to load settings: error running server function: forbidden</p></div>"#,
            header(
                "Temp voice",
                "On-demand voice channels created from a join-to-create channel."
            ),
        )
    );
    assert_eq!(app.discord.count("POST", "/guilds/8/channels"), 0);

    let html = app.page("/guild/abc/settings").await.unwrap();
    assert!(html.contains(
        r#"<p class="error">Failed to load settings: error running server function: invalid guild id</p>"#
    ), "{html}");

    let html = app.page("/guild/9/settings").await.unwrap();
    assert!(
        html.contains(&select(
            "Rules Channel",
            "rules_channel_id",
            "",
            TEXT_OPTIONS
        )),
        "{html}"
    );
    assert!(html.contains(&format!(
        r#"<div class="setting-field"><label for="field-artist_role_id">Artist Role</label><div class="select"><select class="input" id="field-artist_role_id" aria-describedby="field-artist_role_id-help" disabled=""><option selected="">(not set)</option></select>{CHEVRON}</div><input type="hidden" name="artist_role_id" value=""><p class="field-hint field-warning" id="field-artist_role_id-help">Couldn't reach Discord; the role list is unavailable. Saving keeps the current value. (error running server function: "#
    )), "{html}");
    assert_eq!(count(&html, "field-warning"), 3, "{html}");

    let locked = |label: &str, name: &str| {
        format!(
            r#"<div class="setting-field"><label for="field-{name}">{label}</label><div class="select"><select class="input" id="field-{name}" aria-describedby="field-{name}-help" disabled=""><option selected="">(not set)</option></select>{CHEVRON}</div><input type="hidden" name="{name}" value=""><p class="field-hint field-warning" id="field-{name}-help">Couldn't reach Discord; the channel list is unavailable. Saving keeps the current value. (error running server function: "#
        )
    };
    let html = app.page("/guild/10/settings").await.unwrap();
    assert!(html.contains(&locked("Rules Channel", "rules_channel_id")), "{html}");
    assert!(html.contains(ROLE_OPTIONS), "{html}");
    assert_eq!(count(&html, "field-warning"), 3, "{html}");
    let html = app
        .submit(
            "/guild/10/settings/general",
            "guild=10&rules_channel_id=&general_channel_id=&spoiler_channel_id=",
            StatusCode::OK,
        )
        .await
        .unwrap();
    assert!(html.contains(&format!("Channels</legend>{}", saved())), "{html}");
    let html = app.page("/guild/10/settings/temp-voice").await.unwrap();
    assert!(html.contains(&locked("Category", "temp_voice_category")), "{html}");
    assert!(
        html.contains(&locked("Creator Channel", "temp_voice_creator_channel")),
        "{html}"
    );
    let html = app
        .submit(
            "/guild/10/settings/temp-voice",
            "guild=10&temp_voice_category=&temp_voice_creator_channel=",
            StatusCode::OK,
        )
        .await
        .unwrap();
    assert!(
        html.contains(&format!(r#"<fieldset class="settings-section">{}"#, saved())),
        "{html}"
    );

    pool.close().await;
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
