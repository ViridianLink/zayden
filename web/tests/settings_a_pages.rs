//! Server settings and the AI Chat, Family, Honeypot, LFG, Music and Temp
//! voice pages through the app router, against Postgres and a stand-in
//! Discord API.
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

impl Harness {
    /// Posts `body`, checks the 303 to `target` (fragment included), and opens
    /// the page there carrying the flash the redirect set.
    async fn redirected(
        &self,
        path: &str,
        body: &str,
        target: &str,
    ) -> TestResult<String> {
        let response = self.post(path, body, Some(MEMBER)).await?;
        if response.status() != StatusCode::SEE_OTHER
            || location(&response) != Some(target)
        {
            let got = (response.status(), location(&response).map(str::to_owned));
            let html = body_text(response).await?;
            return Err(format!("{path} {body} answered {got:?}: {html}").into());
        }
        let flash = flash_cookie(&response).ok_or("the redirect set no flash")?;
        let page = target.split('#').next().unwrap_or(target);
        let response = self.get(page, Some(&format!("{MEMBER}; {flash}"))).await?;
        if response.status() != StatusCode::OK {
            return Err(format!("{page} answered {}", response.status()).into());
        }
        body_text(response).await
    }
}

/// The markup of the `tag` element whose `id` is `id`, through its end tag.
fn element<'a>(html: &'a str, tag: &str, id: &str) -> Option<&'a str> {
    let at = html.find(&format!(r#"id="{id}""#))?;
    let start = html.get(..at)?.rfind(&format!("<{tag}"))?;
    let rest = html.get(start..)?;
    let end = rest.find(&format!("</{tag}>")).map_or_else(
        || rest.find('>').map(|end| end + 1),
        |end| Some(end + tag.len() + 3),
    )?;
    rest.get(..end)
}

/// The value of the selected option of the select with id `id`.
fn selected<'a>(html: &'a str, id: &str) -> Option<&'a str> {
    let select = element(html, "select", id)?;
    let option = select.get(..select.find(r#" selected="">"#)?)?;
    let option = option.get(option.rfind("<option")?..)?;
    option
        .split_once(r#"value=""#)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(value, _)| value)
}

/// The `value` attribute of the input with id `id`.
fn value<'a>(html: &'a str, id: &str) -> Option<&'a str> {
    let input = element(html, "input", id)?;
    input
        .split_once(r#" value=""#)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(value, _)| value)
}

/// The text between `open` and the next `close`.
fn between<'a>(html: &'a str, open: &str, close: &str) -> Option<&'a str> {
    html.split_once(open)?.1.split_once(close).map(|(text, _)| text)
}

/// The inline error of field `id`.
fn field_error<'a>(html: &'a str, id: &str) -> Option<&'a str> {
    between(html, &format!(r#"<p class="field-error" id="{id}-error">"#), "</p>")
}

/// The reason at the top of form `form`.
fn summary<'a>(html: &'a str, form: &str) -> Option<&'a str> {
    between(
        html,
        &format!(
            r#"<div class="error" id="{form}-summary" role="alert" tabindex="-1" autofocus="">"#
        ),
        "</div>",
    )
}

/// The result line in the save bar of form `form`.
fn bar_flash<'a>(html: &'a str, form: &str) -> Option<&'a str> {
    let form = element(html, "form", form)?;
    between(
        form.split_once(r#"<div class="save-bar""#)?.1,
        r#"<span class="flash-text">"#,
        "</span>",
    )
}

/// The result line at the top of the page.
fn top_flash(html: &str) -> Option<&str> {
    let region =
        between(html, r#"<div class="flash-region" role="status">"#, "</div>")?;
    between(region, r#"<span class="flash-text">"#, "</span>")
}

/// Element ids that appear more than once.
fn duplicate_ids(html: &str) -> Vec<String> {
    let mut ids: Vec<&str> = html
        .split(r#" id=""#)
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(id, _)| id))
        .collect();
    ids.sort_unstable();
    let mut duplicates: Vec<String> = ids
        .windows(2)
        .filter_map(|pair| match pair {
            [first, second] if first == second => Some((*first).to_owned()),
            _ => None,
        })
        .collect();
    duplicates.dedup();
    duplicates
}

/// Every control in `main` has a label bound to it.
fn assert_labelled(html: &str, ids: &[&str]) {
    for id in ids {
        assert!(
            html.contains(&format!(r#"for="{id}""#)),
            "no label for {id}: {html}"
        );
        assert!(html.contains(&format!(r#"id="{id}""#)), "no control {id}: {html}");
    }
}

fn title(label: &str) -> String {
    format!("<title>{label} - Zayden Dashboard</title>")
}

#[sqlx::test(migrations = "../migrations")]
async fn every_page_renders_its_stored_settings(
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
    assert_eq!(settings.channels.get(7).await.unwrap().rules_channel_id, Some(100));

    let html = app.page("/guild/7/settings").await.unwrap();
    assert!(html.contains(&title("Server settings")), "{html}");
    assert_eq!(
        count(
            &html,
            r#"<a href="/guild/7/settings" class="nav-link" aria-current="page">"#
        ),
        2,
        "{html}"
    );
    let main = main_content(&html);
    assert!(main.starts_with(r#"<div class="page"><div class="page-header"><div><h1>Server settings</h1><p class="page-lead">Server-wide channels and roles the rest of Zayden points at.</p></div></div><div class="flash-region" role="status"></div><form id="server-settings" method="post" action="/guild/7/settings" data-pending="" data-dirty-guard=""><input type="hidden" name="guild" value="7"><fieldset class="settings-section"><legend>Channels</legend>"#), "{main}");
    assert_eq!(count(main, "<form"), 1, "one form, one save: {main}");
    assert_eq!(count(main, r#"class="save-bar""#), 1, "{main}");
    assert!(!main.contains("lamp"), "server settings has no module lamp: {main}");
    assert!(main.contains(r#"<p class="field-hint">Zayden's role must be above the roles it assigns. In Discord, drag it above them under Server Settings &gt; Roles.</p>"#) || main.contains("Zayden's role must be above the roles it assigns."), "{main}");
    assert_eq!(selected(main, "server-settings-rules-channel-id"), Some("100"));
    assert_eq!(selected(main, "server-settings-general-channel-id"), Some(""));
    assert_eq!(selected(main, "server-settings-spoiler-channel-id"), Some("999"));
    assert!(
        main.contains(r#"<option value="999" selected="">Unknown (999)</option>"#),
        "{main}"
    );
    assert_eq!(selected(main, "server-settings-sleep-role-id"), Some("201"));
    assert_labelled(main, &[
        "server-settings-rules-channel-id",
        "server-settings-general-channel-id",
        "server-settings-spoiler-channel-id",
        "server-settings-artist-role-id",
        "server-settings-sleep-role-id",
        "server-settings-verified-role-id",
    ]);
    assert_eq!(duplicate_ids(&html), Vec::<String>::new());

    settings
        .ai
        .update(7, |p| {
            p.enabled = true;
            p.channel_id = Some(101);
        })
        .await
        .unwrap();
    let html = app.page("/guild/7/ai").await.unwrap();
    assert!(html.contains(&title("AI Chat")), "{html}");
    assert_eq!(
        count(
            &html,
            r#"<a href="/guild/7/ai" class="nav-link" aria-current="page"><span>AI Chat</span></a>"#
        ),
        2,
        "{html}"
    );
    let main = main_content(&html);
    assert!(main.contains(r#"<div class="settings-actions"><span class="lamp-status"><span class="lamp lamp-on" aria-hidden="true"></span><span class="lamp-text">On</span></span></div>"#), "the lamp shows the setting, with no second control: {main}");
    assert!(!main.contains(r#"role="switch""#), "{main}");
    assert_eq!(selected(main, "ai-settings-enabled"), Some("true"));
    assert_eq!(selected(main, "ai-settings-channel-id"), Some("101"));
    assert_labelled(main, &["ai-settings-enabled", "ai-settings-channel-id"]);
    assert_eq!(duplicate_ids(&html), Vec::<String>::new());

    let html = app.page("/guild/7/family").await.unwrap();
    assert!(html.contains(&title("Family")), "{html}");
    let main = main_content(&html);
    assert!(main.contains(r#"<p class="field-hint">Not synced yet: this module's state appears once Zayden is in the server and has synced its commands.</p>"#), "{main}");
    assert!(main.contains(r#"<span class="lamp lamp-sync" aria-hidden="true"></span><span class="lamp-text">Not synced</span>"#), "{main}");
    assert!(!main.contains(r#"role="switch""#), "no switch before a sync: {main}");
    assert_eq!(value(main, "family-settings-max-partners"), Some("1"));
    assert!(
        main.contains(
            r#"type="number" name="max_partners" value="1" min="1" step="1""#
        ),
        "{main}"
    );
    assert!(main.contains(r#"<p class="field-help" id="family-settings-max-partners-help">At least 1.</p>"#), "{main}");

    let honeypot = settings.honeypot.get(7).await.unwrap();
    assert_eq!(honeypot.purge_seconds, 86_400);
    let html = app.page("/guild/7/honeypot").await.unwrap();
    assert!(html.contains(&title("Honeypot")), "{html}");
    let main = main_content(&html);
    assert_eq!(value(main, "honeypot-settings-purge-seconds"), Some("86400"));
    assert!(main.contains(r#"min="0" max="604800""#), "{main}");
    assert_eq!(selected(main, "honeypot-settings-exempt-admins"), Some("false"));
    assert_labelled(main, &[
        "honeypot-settings-channel-id",
        "honeypot-settings-purge-seconds",
        "honeypot-settings-exempt-admins",
        "honeypot-settings-exempt-role-id",
    ]);

    settings
        .lfg
        .update(7, |p| {
            p.lfg_role_id = Some(200);
            p.lfg_scheduled_thread_id = Some(300);
        })
        .await
        .unwrap();
    let html = app.page("/guild/7/lfg").await.unwrap();
    assert!(html.contains(&title("LFG")), "{html}");
    let main = main_content(&html);
    assert!(!main.contains("lamp"), "LFG has no module: {main}");
    assert_eq!(selected(main, "lfg-settings-lfg-role-id"), Some("200"));
    assert_eq!(value(main, "lfg-settings-lfg-scheduled-thread-id"), Some("300"));
    assert!(main.contains(r#"inputmode="numeric" pattern="[0-9]*""#), "{main}");

    app.app.modules.set(7, "music", true).await.unwrap();
    let html = app.page("/guild/7/music").await.unwrap();
    assert!(html.contains(&title("Music")), "{html}");
    let main = main_content(&html);
    assert!(main.contains(r#"<form class="settings-actions" method="post" action="/guild/7/music/module" data-pending=""><input type="hidden" name="guild" value="7"><span class="lamp-status"><span class="lamp lamp-on" aria-hidden="true"></span><span class="lamp-text" data-pending-text="Saving…">On</span></span><button type="submit" class="switch" role="switch" aria-checked="true" aria-label="Music module" name="enabled" value="false"><span class="switch-thumb" aria-hidden="true"></span></button></form>"#), "{main}");
    assert_eq!(value(main, "music-settings-auto-disconnect-secs"), Some("120"));
    assert_eq!(selected(main, "music-settings-announce-now-playing"), Some("true"));

    settings
        .temp_voice
        .update(7, |p| p.temp_voice_category = Some(102))
        .await
        .unwrap();
    let html = app.page("/guild/7/temp-voice").await.unwrap();
    assert!(html.contains(&title("Temp voice")), "{html}");
    let main = main_content(&html);
    assert_eq!(
        selected(main, "temp-voice-settings-temp-voice-category"),
        Some("102")
    );
    assert_eq!(selected(main, "temp-voice-create-temp-voice-category"), Some("102"));
    assert_eq!(
        selected(main, "temp-voice-settings-temp-voice-creator-channel"),
        Some("")
    );
    assert!(main.contains(r#"<form id="temp-voice-create" method="post" action="/guild/7/temp-voice/create" data-pending="">"#), "{main}");
    assert!(main.contains(r#"<button type="submit" class="btn btn-secondary" data-pending-label="Creating…">Create creator channel</button>"#), "{main}");
    assert_eq!(
        duplicate_ids(&html),
        Vec::<String>::new(),
        "two forms post the same field name"
    );

    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 0);
    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn saves_redirect_with_the_result_in_the_save_bar(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let settings = &app.app.settings;

    let html = app
        .redirected(
            "/guild/7/settings",
            "guild=7&rules_channel_id=101&general_channel_id=&spoiler_channel_id=104&artist_role_id=&sleep_role_id=200&verified_role_id=",
            "/guild/7/settings#server-settings",
        )
        .await
        .unwrap();
    let channels = settings.channels.get(7).await.unwrap();
    assert_eq!(
        (
            channels.rules_channel_id,
            channels.general_channel_id,
            channels.spoiler_channel_id
        ),
        (Some(101), None, Some(104))
    );
    assert_eq!(settings.roles.get(7).await.unwrap().sleep_role_id, Some(200));
    assert_eq!(bar_flash(&html, "server-settings"), Some("Server settings saved."));
    assert_eq!(top_flash(&html), None);
    assert_eq!(selected(&html, "server-settings-rules-channel-id"), Some("101"));
    assert!(
        html.contains(r#"<p class="flash flash-success" data-flash="">"#),
        "{html}"
    );
    let html = app.page("/guild/7/settings").await.unwrap();
    assert_eq!(bar_flash(&html, "server-settings"), None, "the flash shows once");

    let html = app
        .redirected(
            "/guild/7/ai",
            "guild=7&enabled=true&channel_id=100",
            "/guild/7/ai#ai-settings",
        )
        .await
        .unwrap();
    assert_eq!(bar_flash(&html, "ai-settings"), Some("AI Chat settings saved."));
    let ai = settings.ai.get(7).await.unwrap();
    assert!(ai.enabled);
    assert_eq!(ai.channel_id, Some(100));

    let html = app
        .redirected(
            "/guild/7/family",
            "guild=7&max_partners=0",
            "/guild/7/family#family-settings",
        )
        .await
        .unwrap();
    assert_eq!(settings.family.get(7).await.unwrap().max_partners, 1);
    assert_eq!(bar_flash(&html, "family-settings"), Some("Family settings saved."));
    assert_eq!(
        value(&html, "family-settings-max-partners"),
        Some("1"),
        "the page shows what was stored"
    );

    let html = app
        .redirected(
            "/guild/7/honeypot",
            "guild=7&channel_id=100&exempt_admins=true&exempt_role_id=&purge_seconds=999999",
            "/guild/7/honeypot#honeypot-settings",
        )
        .await
        .unwrap();
    let honeypot = settings.honeypot.get(7).await.unwrap();
    assert_eq!(
        (honeypot.channel_id, honeypot.exempt_admins, honeypot.purge_seconds),
        (Some(100), true, 604_800)
    );
    assert_eq!(value(&html, "honeypot-settings-purge-seconds"), Some("604800"));

    app.redirected(
        "/guild/7/lfg",
        "guild=7&lfg_channel_id=104&lfg_role_id=201&lfg_scheduled_thread_id=300",
        "/guild/7/lfg#lfg-settings",
    )
    .await
    .unwrap();
    let lfg = settings.lfg.get(7).await.unwrap();
    assert_eq!(
        (lfg.lfg_channel_id, lfg.lfg_role_id, lfg.lfg_scheduled_thread_id),
        (Some(104), Some(201), Some(300))
    );

    let html = app
        .redirected(
            "/guild/7/music",
            "guild=7&dj_role_id=200&auto_disconnect_secs=60&announce_now_playing=false&announce_channel_id=101",
            "/guild/7/music#music-settings",
        )
        .await
        .unwrap();
    let music = settings.music.get(7).await.unwrap();
    assert_eq!(
        (music.dj_role_id, music.auto_disconnect_secs, music.announce_now_playing),
        (Some(200), 60, false)
    );
    assert_eq!(
        selected(&html, "music-settings-announce-now-playing"),
        Some("false")
    );

    let html = app
        .redirected(
            "/guild/7/temp-voice",
            "guild=7&temp_voice_category=102&temp_voice_creator_channel=103",
            "/guild/7/temp-voice#temp-voice-settings",
        )
        .await
        .unwrap();
    assert_eq!(
        settings.temp_voice.get(7).await.unwrap().temp_voice_creator_channel,
        Some(103)
    );
    assert_eq!(
        bar_flash(&html, "temp-voice-settings"),
        Some("Temp voice settings saved.")
    );
    assert_eq!(count(&html, "flash-text"), 1, "{html}");
    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 0);

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
    let html = app
        .redirected(
            "/guild/7/temp-voice/create",
            "guild=7&temp_voice_category=102",
            "/guild/7/temp-voice#temp-voice-settings",
        )
        .await
        .unwrap();
    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 1);
    let temp_voice = settings.temp_voice.get(7).await.unwrap();
    assert_eq!(
        (temp_voice.temp_voice_category, temp_voice.temp_voice_creator_channel),
        (Some(102), Some(400))
    );
    assert_eq!(
        bar_flash(&html, "temp-voice-settings"),
        Some("Creator channel created.")
    );
    assert_eq!(
        selected(&html, "temp-voice-settings-temp-voice-creator-channel"),
        Some("400")
    );

    for path in [
        "/guild/7/temp-voice?created=1",
        "/guild/7/family?saved=1",
        "/guild/7/settings?added=1",
    ] {
        let html = app.page(path).await.unwrap();
        assert_eq!(
            count(&html, "flash-text"),
            0,
            "legacy flags are ignored: {path}"
        );
    }
    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 1);

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn failures_re_render_with_the_reason_at_the_form_and_the_field(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let settings = &app.app.settings;
    settings.channels.update(7, |p| p.rules_channel_id = Some(101)).await.unwrap();

    let html = app
        .submit(
            "/guild/7/settings",
            "guild=7&rules_channel_id=100&general_channel_id=&spoiler_channel_id=&artist_role_id=999&sleep_role_id=200&verified_role_id=",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(html.contains(&title("Server settings")), "{html}");
    assert_eq!(
        summary(&html, "server-settings"),
        Some("Not saved: that role is not in this server")
    );
    assert_eq!(
        settings.channels.get(7).await.unwrap().rules_channel_id,
        Some(101),
        "nothing is written when any id is foreign"
    );
    assert_eq!(settings.roles.get(7).await.unwrap().sleep_role_id, None);
    assert_eq!(
        selected(&html, "server-settings-rules-channel-id"),
        Some("100"),
        "the typed values stay"
    );
    assert_eq!(selected(&html, "server-settings-artist-role-id"), Some("999"));
    assert_eq!(selected(&html, "server-settings-sleep-role-id"), Some("200"));
    assert!(!html.contains("error running server function"), "{html}");

    for (body, message, field) in [
        (
            "guild=8&rules_channel_id=&general_channel_id=&spoiler_channel_id=&artist_role_id=&sleep_role_id=&verified_role_id=",
            "invalid value for `guild`",
            None,
        ),
        (
            "guild=7&rules_channel_id=&general_channel_id=&spoiler_channel_id=&artist_role_id=&sleep_role_id=&verified_role_id=&x=1",
            "unknown field `x`",
            None,
        ),
        (
            "guild=7&rules_channel_id=&general_channel_id=&spoiler_channel_id=&artist_role_id=&sleep_role_id=",
            "missing field `verified_role_id`",
            Some("server-settings-verified-role-id"),
        ),
        (
            "guild=7&rules_channel_id=&rules_channel_id=&general_channel_id=&spoiler_channel_id=&artist_role_id=&sleep_role_id=&verified_role_id=",
            "duplicate field `rules_channel_id`",
            Some("server-settings-rules-channel-id"),
        ),
    ] {
        let html = app
            .submit("/guild/7/settings", body, StatusCode::UNPROCESSABLE_ENTITY)
            .await
            .unwrap();
        assert_eq!(
            summary(&html, "server-settings"),
            Some(format!("Not saved: {message}").as_str()),
            "{body}"
        );
        if let Some(id) = field {
            assert_eq!(field_error(&html, id), Some(message), "{body}: {html}");
            assert!(
                html.contains(&format!(
                    r#"aria-describedby="{id}-error" aria-invalid="true""#
                )),
                "{body}: {html}"
            );
        } else {
            assert!(!html.contains("field-error"), "{body}: {html}");
        }
    }
    assert_eq!(settings.channels.get(7).await.unwrap().rules_channel_id, Some(101));

    let html = app
        .submit(
            "/guild/7/honeypot",
            "guild=7&channel_id=abc&exempt_admins=true&exempt_role_id=&purge_seconds=",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "honeypot-settings"),
        Some(
            "Not saved: `abc` is not a valid channel id. Leave the field blank to clear it."
        )
    );
    assert_eq!(selected(&html, "honeypot-settings-exempt-admins"), Some("true"));
    assert!(!settings.honeypot.get(7).await.unwrap().exempt_admins);

    let html = app
        .submit(
            "/guild/7/lfg",
            "guild=7&lfg_channel_id=&lfg_role_id=&lfg_scheduled_thread_id=301",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "lfg-settings"),
        Some("Not saved: that channel is not in this server")
    );
    assert_eq!(value(&html, "lfg-settings-lfg-scheduled-thread-id"), Some("301"));

    let html = app
        .submit(
            "/guild/7/temp-voice/create",
            "guild=7&temp_voice_category=",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "temp-voice-create"),
        Some("Not created: select a category first")
    );
    assert_eq!(
        field_error(&html, "temp-voice-create-temp-voice-category"),
        Some("select a category first")
    );
    assert_eq!(
        summary(&html, "temp-voice-settings"),
        None,
        "the other form is untouched"
    );
    for (body, message) in [
        ("guild=8&temp_voice_category=102", "invalid value for `guild`"),
        (
            "guild=7&temp_voice_category=102&artist_role_id=",
            "unknown field `artist_role_id`",
        ),
    ] {
        let html = app
            .submit(
                "/guild/7/temp-voice/create",
                body,
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await
            .unwrap();
        assert_eq!(
            summary(&html, "temp-voice-create"),
            Some(format!("Not created: {message}").as_str()),
            "{body}"
        );
    }
    let html = app
        .submit(
            "/guild/7/temp-voice",
            "guild=7&temp_voice_category=102&temp_voice_creator_channel=103&x=1",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "temp-voice-settings"),
        Some("Not saved: unknown field `x`")
    );
    assert_eq!(app.discord.count("POST", "/guilds/7/channels"), 0);

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn the_header_switch_turns_a_command_module_on_and_off(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let modules = &app.app.modules;
    modules.set(7, "music", true).await.unwrap();

    let html = app
        .redirected(
            "/guild/7/music/module",
            "guild=7&enabled=false",
            "/guild/7/music",
        )
        .await
        .unwrap();
    modules.refresh(7).await.unwrap();
    assert_eq!(modules.states(7).await.unwrap().get("music").copied(), Some(false));
    assert_eq!(top_flash(&html), Some("Music turned off."));
    assert!(html.contains(r#"aria-checked="false" aria-label="Music module" name="enabled" value="true""#), "{html}");
    assert!(
        html.contains(r#"<span class="lamp lamp-off" aria-hidden="true"></span>"#),
        "{html}"
    );

    for (path, page, label) in [
        ("/guild/7/family/module", "/guild/7/family", "Family"),
        ("/guild/7/honeypot/module", "/guild/7/honeypot", "Honeypot"),
    ] {
        let html = app.redirected(path, "guild=7&enabled=true", page).await.unwrap();
        assert_eq!(
            top_flash(&html),
            Some(format!("{label} turned on.").as_str()),
            "{path}"
        );
        assert!(
            html.contains(&format!(
                r#"aria-checked="true" aria-label="{label} module""#
            )),
            "{html}"
        );
    }

    let html = app
        .submit(
            "/guild/7/music/module",
            "guild=7&enabled=on",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(html.contains(&title("Music")), "{html}");
    assert!(html.contains(r#"<p class="error" role="alert">Not changed: invalid value for `enabled`</p>"#), "{html}");
    let html = app
        .submit(
            "/guild/7/music/module",
            "guild=8&enabled=true",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(html.contains("Not changed: invalid value for `guild`"), "{html}");
    modules.refresh(7).await.unwrap();
    assert_eq!(modules.states(7).await.unwrap().get("music").copied(), Some(false));

    let response = app
        .post("/guild/7/music/module", "guild=7&enabled=true", None)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn pages_report_what_they_cannot_load(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();

    for path in ["/guild/7/settings", "/guild/7/ai", "/guild/7/temp-voice"] {
        let response = app.get(path, None).await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(location(&response), Some("/login"), "{path}");
    }
    let response =
        app.post("/guild/7/family", "guild=7&max_partners=3", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));
    assert_eq!(app.app.settings.family.get(7).await.unwrap().max_partners, 1);

    let html = app.page("/guild/8/music").await.unwrap();
    let main = main_content(&html);
    assert!(main.contains(r#"<section class="error-panel" role="alert" aria-labelledby="load-error-title"><h2 class="error-title" id="load-error-title">Couldn't load these settings</h2><p class="error-text">You need Manage Server in this server to change its settings.</p><div class="error-actions"><a href="/guild/8/music" class="btn btn-primary">Try again</a><a href="/guilds" class="btn btn-secondary">Back to servers</a></div></section>"#), "{main}");
    assert!(!main.contains("<form"), "{main}");

    let html = app
        .submit(
            "/guild/8/family",
            "guild=8&max_partners=3",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(summary(&html, "page"), Some("Not saved: forbidden"));
    assert!(html.contains("Couldn't load these settings"), "{html}");
    let html = app
        .submit(
            "/guild/8/temp-voice/create",
            "guild=8&temp_voice_category=102",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(html.contains("Couldn't load these settings"), "{html}");
    assert_eq!(app.discord.count("POST", "/guilds/8/channels"), 0);

    let html = app.page("/guild/abc/settings").await.unwrap();
    assert!(html.contains(r#"<p class="error-text">That address doesn't name a Discord server.</p>"#), "{html}");

    let locked = |id: &str, name: &str, list: &str| {
        format!(
            r#"<select class="input" id="{id}" aria-describedby="{id}-help" disabled=""><option selected="">(not set)</option></select><span class="select-chevron">"#
        ) + &format!(
            r#"<input type="hidden" name="{name}" value=""><p class="field-help" id="{id}-help">Couldn't reach Discord, so the {list} list is unavailable. Saving keeps the current value.</p>"#
        )
    };
    let html = app.page("/guild/9/settings").await.unwrap();
    assert_eq!(selected(&html, "server-settings-rules-channel-id"), Some(""));
    let artist = locked("server-settings-artist-role-id", "artist_role_id", "role");
    let (select, rest) =
        artist.split_once(r#"<span class="select-chevron">"#).unwrap();
    assert!(html.contains(select), "{html}");
    assert!(html.contains(rest), "{html}");
    assert_eq!(count(&html, "Saving keeps the current value."), 3, "{html}");

    let html = app.page("/guild/10/settings").await.unwrap();
    let rules =
        locked("server-settings-rules-channel-id", "rules_channel_id", "channel");
    let (select, rest) =
        rules.split_once(r#"<span class="select-chevron">"#).unwrap();
    assert!(html.contains(select), "{html}");
    assert!(html.contains(rest), "{html}");
    assert_eq!(selected(&html, "server-settings-artist-role-id"), Some(""));
    assert!(!html.contains("error running server function"), "{html}");
    app.redirected(
        "/guild/10/settings",
        "guild=10&rules_channel_id=&general_channel_id=&spoiler_channel_id=&artist_role_id=&sleep_role_id=&verified_role_id=",
        "/guild/10/settings#server-settings",
    )
    .await
    .unwrap();
    app.redirected(
        "/guild/10/temp-voice",
        "guild=10&temp_voice_category=&temp_voice_creator_channel=",
        "/guild/10/temp-voice#temp-voice-settings",
    )
    .await
    .unwrap();

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
