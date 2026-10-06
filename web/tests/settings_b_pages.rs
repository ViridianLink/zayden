//! The guild settings page's YouTube, Patreon and Support sections through
//! the app router, against Postgres and a stand-in Discord API.
//!
//! Sessions and Discord guild lists come from seeded caches. The bot's
//! Discord client talks to a local server that answers the channel, role,
//! member and user endpoints the sections use. Stored values are read
//! directly before the markup is checked, so a database fault fails with its
//! own error.

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
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::state::{DiscordState, SessionIdentity, WebState};
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
const SAVE: &str = r#"<div class="form-actions"><button type="submit" class="btn btn-primary" data-pending-label="Saving…">Save</button></div>"#;

const TEXT_OPTIONS: &str = r#"<option value="100"># rules</option><option value="101">📢 news</option><option value="104">💬 help</option>"#;

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

fn forum_json(id: u64, name: &str, position: u32, tag: (u64, &str)) -> String {
    format!(
        r#"{{"id":"{id}","type":15,"name":"{name}","position":{position},"guild_id":"7","available_tags":[{{"id":"{}","name":"{}","moderated":false,"emoji_id":null,"emoji_name":null}}]}}"#,
        tag.0, tag.1
    )
}

fn member_json(id: u64, name: &str, nick: Option<&str>) -> String {
    let nick = nick.map_or_else(|| "null".to_owned(), |nick| format!(r#""{nick}""#));
    format!(
        r#"{{"communication_disabled_until":null,"deaf":false,"flags":0,"joined_at":null,"mute":false,"nick":{nick},"roles":[],"user":{{"accent_color":null,"avatar":null,"avatar_decoration":null,"avatar_decoration_data":null,"banner":null,"discriminator":"0","id":"{id}","username":"{name}"}}}}"#
    )
}

/// Guild 7: text, announcement and forum channels (the forum carries one
/// tag), two roles besides @everyone, the bot (900, no permissions of its
/// own) and helper 501. Only #news (101) lets @everyone post. Guild 9 lists
/// its channels but fails its roles.
fn serve_guilds(discord: &Discord) {
    let channels = list(&[
        channel_json(100, "rules", 0, 0),
        channel_json(101, "news", 5, 1),
        forum_json(104, "help", 4, (600, "Solved")),
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
        "/users/@me",
        200,
        r#"{"accent_color":null,"avatar":null,"banner":null,"bot":true,"discriminator":"0","id":"900","mfa_enabled":false,"username":"Zayden"}"#,
    );
    discord.reply(
        "GET",
        "/guilds/7/members/900",
        200,
        &member_json(900, "Zayden", None),
    );
    discord.reply(
        "GET",
        "/guilds/7/members/501",
        200,
        &member_json(501, "helper", Some("Helper One")),
    );
    discord.reply(
        "GET",
        "/channels/100",
        200,
        r#"{"id":"100","type":0,"name":"rules","guild_id":"7","permission_overwrites":[]}"#,
    );
    discord.reply(
        "GET",
        "/channels/101",
        200,
        r#"{"id":"101","type":5,"name":"news","guild_id":"7","permission_overwrites":[{"id":"7","type":0,"allow":"3072","deny":"0"}]}"#,
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
        .join(format!("web-settings-b-assets-{}", std::process::id()));
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
    };
    state
        .sessions
        .insert("member-token".to_owned(), SessionIdentity {
            user_id: 41,
            access_token: "member-token-access".to_owned(),
        })
        .await;
    state
        .discord
        .user_guilds
        .insert(
            41,
            Arc::from([
                guild(7, Permissions::MANAGE_GUILD),
                guild(8, Permissions::SEND_MESSAGES),
                guild(9, Permissions::ADMINISTRATOR),
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

impl Harness {
    /// Posts `body`, checks the 303 to `target`, and returns the page there.
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
        self.page(target).await
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
    html.split_once(r#"<main class="app-main">"#)
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
        r#"<div class="setting-field"><label>{label}</label><div class="select"><select class="input" name="{name}"><option value=""{none}>(not set)</option>{options}</select>{CHEVRON}</div></div>"#
    )
}

fn toggle(label: &str, name: &str, on: bool) -> String {
    let (yes, no) =
        if on { (r#" selected="""#, "") } else { ("", r#" selected="""#) };
    format!(
        r#"<div class="setting-field"><label>{label}</label><div class="select"><select class="input" name="{name}"><option value="true"{yes}>Enabled</option><option value="false"{no}>Disabled</option></select>{CHEVRON}</div></div>"#
    )
}

fn alert(class: &str, role: &str, message: &str) -> String {
    format!(
        r#"<div class="alert {class}" role="{role}"><span>{message}</span><button type="button" class="alert-dismiss" aria-label="Dismiss">{ICON_OPEN}<path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg></button></div>"#
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

const TAG_OPTIONS: &str = r#"<option value="600">#help / Solved</option>"#;

fn connect_youtube_json() -> youtube::model::OwnChannel {
    youtube::model::OwnChannel {
        id: "UC123".to_owned(),
        title: "Seed Channel".to_owned(),
        uploads_playlist_id: "UU123".to_owned(),
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn provider_sections_follow_the_connection(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let db = &app.app.db;

    assert!(youtube::YoutubeConnection::select(db, 7).await.unwrap().is_none());
    let html = app.page("/guild/7/settings/youtube").await.unwrap();
    assert!(
        html.contains("<title>YouTube settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<fieldset class="settings-section"><p class="page-lead">No YouTube channel is connected. The channel's owner signs in with Google once to prove it is theirs; Zayden keeps no access to the account afterwards.</p><div class="settings-actions"><a class="btn btn-primary" href="/youtube/connect?guild=7" rel="external">Connect YouTube</a></div></fieldset></div>"#,
            header(
                "YouTube",
                "Connect a YouTube channel and choose where its uploads are announced."
            ),
        )
    );

    let html = app.page("/guild/7/settings/youtube?youtube=declined").await.unwrap();
    assert!(
        main_content(&html).contains(&alert(
            "warning",
            "status",
            "Authorisation was cancelled, so nothing changed."
        )),
        "{html}"
    );
    let html = app.page("/guild/7/settings/youtube?youtube=bogus").await.unwrap();
    assert!(!main_content(&html).contains("class=\"alert"), "{html}");

    youtube::YoutubeConnection::connect(db, 7, &connect_youtube_json(), 41, "s")
        .await
        .unwrap();
    assert!(youtube::YoutubeConnection::select(db, 7).await.unwrap().is_some());
    let html =
        app.page("/guild/7/settings/youtube?youtube=connected").await.unwrap();
    let action = "/guild/7/settings/youtube?youtube=connected";
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}{}<fieldset class="settings-section"><p class="page-lead">Connected to Seed Channel.</p><p class="page-lead">Push notifications are not confirmed yet, so uploads arrive on the 15-minute poll. They are renewed automatically.</p><div class="settings-actions"><a class="btn btn-secondary" href="/youtube/connect?guild=7" rel="external">Reconnect YouTube</a><form method="post" action="{action}" data-pending=""><input type="hidden" name="guild" value="7">{}</form></div><form method="post" action="{action}" data-pending=""><input type="hidden" name="guild" value="7">{}{SAVE}</form><p class="page-lead">Leave the channel unset to stop announcing without disconnecting. Only public uploads are announced; videos older than two days when Zayden first sees them are skipped, so connecting never floods a channel with the back catalogue.</p></fieldset></div>"#,
            header(
                "YouTube",
                "Connect a YouTube channel and choose where its uploads are announced."
            ),
            alert(
                "success",
                "status",
                "YouTube connected. Choose where its uploads should be announced."
            ),
            confirm(
                "Disconnect",
                "Zayden stops announcing this channel's uploads. Reconnecting needs the channel owner to sign in with Google again.",
                "Disconnect YouTube"
            ),
            select("Announcement Channel", "channel_id", "", TEXT_OPTIONS),
        )
    );

    let html = app
        .submit(action, "guild=7&channel_id=100", StatusCode::UNPROCESSABLE_ENTITY)
        .await
        .unwrap();
    assert!(main_content(&html).contains(&format!(
        "{}<form",
        not_saved(
            "Zayden can't post in #rules: it needs View Channel and Send Messages there. Allow them for the bot's role in the channel's permissions, then save again."
        )
    )), "{html}");
    assert!(
        main_content(&html).contains(&select(
            "Announcement Channel",
            "channel_id",
            "100",
            TEXT_OPTIONS
        )),
        "{html}"
    );
    assert!(youtube::YoutubeAnnounceRow::select(db, 7).await.unwrap().is_none());

    let html =
        app.submit(action, "guild=7&channel_id=101", StatusCode::OK).await.unwrap();
    assert_eq!(
        youtube::YoutubeAnnounceRow::select(db, 7)
            .await
            .unwrap()
            .map(|r| r.channel_id),
        Some(101)
    );
    assert!(main_content(&html).contains(&format!("{}<form", saved())), "{html}");
    assert!(
        main_content(&html).contains(&select(
            "Announcement Channel",
            "channel_id",
            "101",
            TEXT_OPTIONS
        )),
        "{html}"
    );

    let html = app
        .submit(
            "/guild/7/settings/youtube",
            "guild=8",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(
        main_content(&html).contains(&alert(
            "error",
            "alert",
            "Failed to disconnect: error running server function: invalid value for `guild`"
        )),
        "{html}"
    );

    let html = app
        .redirected(
            action,
            "guild=7",
            "/guild/7/settings/youtube?youtube=connected&disconnected=1",
        )
        .await
        .unwrap();
    assert!(youtube::YoutubeConnection::select(db, 7).await.unwrap().is_none());
    let main = main_content(&html);
    assert!(
        main.contains(&format!(
            "{}{}<fieldset",
            alert(
                "success",
                "status",
                "YouTube connected. Choose where its uploads should be announced."
            ),
            alert(
                "success",
                "status",
                "YouTube disconnected. Zayden will stop announcing this channel."
            )
        )),
        "{html}"
    );
    assert!(main.contains("Connect YouTube</a>"), "{html}");

    assert!(patreon::PatreonConnection::select(db, 7).await.unwrap().is_none());
    let html = app.page("/guild/7/settings/patreon").await.unwrap();
    assert!(
        html.contains("<title>Patreon settings - Zayden Dashboard</title>"),
        "{html}"
    );
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<fieldset class="settings-section"><p class="page-lead">No Patreon account is connected. The campaign's own creator has to authorise Zayden - the connection reads their posts, so nobody else can grant it.</p><div class="settings-actions"><a class="btn btn-primary" href="/patreon/connect?guild=7" rel="external">Connect Patreon</a></div></fieldset></div>"#,
            header(
                "Patreon",
                "Connect a Patreon campaign and choose where its posts are announced."
            ),
        )
    );

    let tokens = patreon::TokenPair {
        access_token: "a".to_owned(),
        refresh_token: "r".to_owned(),
        expires_in: 3600.0,
    };
    patreon::PatreonConnection::connect(
        db,
        7,
        "camp-1",
        None,
        41,
        &tokens,
        Some(("hook", "secret")),
    )
    .await
    .unwrap();
    patreon::PatreonConnection::disable(db, 7).await.unwrap();
    assert!(patreon::PatreonConnection::select(db, 7).await.unwrap().is_some());
    let html = app.page("/guild/7/settings/patreon").await.unwrap();
    let action = "/guild/7/settings/patreon";
    assert_eq!(
        main_content(&html),
        format!(
            r#"{}<fieldset class="settings-section"><p class="page-lead">Connected to camp-1, but Patreon has rejected the stored authorisation. Reconnect to resume announcements.</p><p class="page-lead">New posts arrive within seconds via a webhook on the creator's account, with a poll every 15 minutes as a safety net.</p><div class="settings-actions"><a class="btn btn-secondary" href="/patreon/connect?guild=7" rel="external">Reconnect Patreon</a><form method="post" action="{action}" data-pending=""><input type="hidden" name="guild" value="7">{}</form></div><form method="post" action="{action}" data-pending=""><input type="hidden" name="guild" value="7">{}{}{SAVE}</form><p class="page-lead">Leave the channel unset to stop announcing without disconnecting the account. Posts published before the first poll are absorbed rather than announced, so connecting never floods a channel with back catalogue.</p></fieldset></div>"#,
            header(
                "Patreon",
                "Connect a Patreon campaign and choose where its posts are announced."
            ),
            confirm(
                "Disconnect",
                "Zayden stops announcing this campaign and drops its webhook on the creator's Patreon account. Reconnecting needs the creator to authorise again.",
                "Disconnect Patreon"
            ),
            select("Announcement Channel", "channel_id", "", TEXT_OPTIONS),
            toggle("Public Posts Only", "public_only", false),
        )
    );

    let html =
        app.page("/guild/7/settings/patreon?patreon=connected").await.unwrap();
    assert!(
        main_content(&html).contains(&format!(
            "{}<fieldset",
            alert(
                "success",
                "status",
                "Patreon connected. Choose where its posts should be announced."
            )
        )),
        "{html}"
    );

    let html = app
        .submit(
            action,
            "guild=7&channel_id=100&public_only=true",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(patreon::PatreonAnnounceRow::select(db, 7).await.unwrap().is_none());
    let main = main_content(&html);
    assert!(
        main.contains(&format!(
            "{}<form",
            not_saved(
                "Zayden can't post in #rules: it needs View Channel and Send Messages there. Allow them for the bot's role in the channel's permissions, then save again."
            )
        )),
        "{html}"
    );
    assert!(
        main.contains(&select(
            "Announcement Channel",
            "channel_id",
            "100",
            TEXT_OPTIONS
        )),
        "{html}"
    );
    assert!(
        main.contains(&toggle("Public Posts Only", "public_only", true)),
        "{html}"
    );

    let html = app
        .submit(action, "guild=7&channel_id=101&public_only=true", StatusCode::OK)
        .await
        .unwrap();
    let announce =
        patreon::PatreonAnnounceRow::select(db, 7).await.unwrap().unwrap();
    assert_eq!((announce.channel_id, announce.public_only), (101, true));
    let main = main_content(&html);
    assert!(main.contains(&format!("{}<form", saved())), "{html}");
    assert!(
        main.contains(&toggle("Public Posts Only", "public_only", true)),
        "{html}"
    );

    let html = app
        .submit(action, "guild=8", StatusCode::UNPROCESSABLE_ENTITY)
        .await
        .unwrap();
    assert!(patreon::PatreonConnection::select(db, 7).await.unwrap().is_some());
    assert!(
        main_content(&html).contains(&format!(
            "{}<fieldset",
            alert(
                "error",
                "alert",
                "Failed to disconnect: error running server function: invalid value for `guild`"
            )
        )),
        "{html}"
    );

    let html = app
        .redirected(action, "guild=7", "/guild/7/settings/patreon?disconnected=1")
        .await
        .unwrap();
    assert!(patreon::PatreonConnection::select(db, 7).await.unwrap().is_none());
    assert!(
        main_content(&html).contains(&format!(
            "{}<fieldset",
            alert(
                "success",
                "status",
                "Patreon disconnected. Zayden will stop announcing this campaign."
            )
        )),
        "{html}"
    );

    let response = app.post(action, "guild=7", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));

    let load_error = r#"<p class="error">Failed to load settings: error running server function: forbidden</p>"#;
    let html = app.page("/guild/8/settings/youtube").await.unwrap();
    assert!(main_content(&html).contains(load_error), "{html}");

    let html = app
        .submit(
            "/guild/8/settings/youtube",
            "guild=8",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    let main = main_content(&html);
    assert!(main.contains(load_error), "{html}");
    assert!(!main.contains("Failed to disconnect"), "{html}");
    assert!(!main.contains("Failed to save"), "{html}");

    let html = app
        .submit(
            "/guild/8/settings/youtube",
            "guild=8&channel_id=101",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(
        main_content(&html)
            .contains(&format!("{}{load_error}", not_saved("forbidden"))),
        "{html}"
    );

    pool.close().await;
}

fn confirm(label: &str, prompt: &str, confirm: &str) -> String {
    format!(
        r#"<details class="confirm"><summary class="btn btn-danger"><span class="confirm-label">{label}</span><span class="confirm-cancel">Cancel</span></summary><div class="confirm-panel"><p class="confirm-prompt">{prompt}</p><button type="submit" class="btn btn-danger">{confirm}</button></div></details>"#
    )
}

fn field(
    label: &str,
    name: &str,
    value: &str,
    pattern: &str,
    hint: Option<&str>,
) -> String {
    let hint = hint.map_or_else(String::new, |hint| {
        format!(r#"<p class="field-hint">{hint}</p>"#)
    });
    format!(
        r#"<div class="setting-field"><label>{label}</label><input class="input" type="text" name="{name}" value="{value}" placeholder="(not set)" pattern="{pattern}">{hint}</div>"#
    )
}

const SEGMENTED_SETTINGS: &str = r#"<div class="segmented" role="tablist"><button type="button" class="seg active">Settings</button><button type="button" class="seg">FAQ</button></div><fieldset class="settings-section">"#;
const SEGMENTED_FAQ: &str = r#"<div class="segmented" role="tablist"><button type="button" class="seg">Settings</button><button type="button" class="seg active">FAQ</button></div><fieldset class="settings-section" hidden="">"#;
const SUPPORT: &str = "/guild/7/settings/support";

async fn support_roles(app: &Harness) -> TestResult<Vec<u64>> {
    Ok(ticket::SupportRoles::ids(&app.app.db, ticket::GuildId::new(7))
        .await?
        .into_iter()
        .map(ticket::RoleId::get)
        .collect())
}

async fn helper_users(app: &Harness) -> TestResult<Vec<u64>> {
    let mut users: Vec<u64> =
        ticket::HelperLinks::list(&app.app.db, ticket::GuildId::new(7))
            .await?
            .into_iter()
            .map(|link| link.user_id.get())
            .collect();
    users.sort_unstable();
    Ok(users)
}

#[sqlx::test(migrations = "../migrations")]
async fn support_settings_save_and_keep_what_was_sent(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let settings = &app.app.settings;

    let support = settings.support.get(7).await.unwrap();
    assert_eq!(
        (support.solved_archive_secs, support.idle_after_secs),
        (60, 172_800)
    );
    let html = app.page(SUPPORT).await.unwrap();
    assert!(
        html.contains("<title>Support settings - Zayden Dashboard</title>"),
        "{html}"
    );
    let main = main_content(&html);
    assert!(
        main.starts_with(&header(
            "Support",
            "Tickets, FAQ and suggestions - where they live and who gets pinged."
        )),
        "{html}"
    );
    assert!(
        main.contains(&format!(
            "{SEGMENTED_SETTINGS}{}{}{}{}{}{SAVE}</form>",
            form("support", "7"),
            select("Support Channel", "support_channel_id", "", TEXT_OPTIONS),
            select("Solved Tag", "solved_tag_id", "", TAG_OPTIONS),
            select("Closed Tag", "closed_tag_id", "", TAG_OPTIONS),
            field(
                "Archive solved posts after (seconds)",
                "solved_archive_secs",
                "60",
                "-?[0-9]*",
                None
            ),
        )),
        "{html}"
    );
    let idle_form = |after: &str| {
        format!(
            "{}{}{}{}{}{SAVE}</form>",
            form("support", "7"),
            toggle("Idle Reminders", "idle_enabled", false),
            field(
                "Remind after (seconds of silence)",
                "idle_after_secs",
                after,
                "[0-9]*",
                Some("Minimum one hour. Default 172800 (48 hours).")
            ),
            toggle("Auto-close Abandoned Posts", "idle_close_enabled", false),
            field(
                "Close after (seconds without a reply to the reminder)",
                "idle_close_after_secs",
                "86400",
                "[0-9]*",
                Some("Minimum one hour. Default 86400 (24 hours).")
            ),
        )
    };
    assert!(main.contains(&idle_form("172800")), "{html}");
    assert!(main.contains(&format!(
        r#"{}<input type="hidden" name="keep_wiki_api_key" value="true">{SAVE}"#,
        r#"<div class="setting-field"><label>Wiki API Key</label><input class="input" type="password" name="wiki_api_key" value="" placeholder="eyJhbGciOiJSUzI1NiIs..." pattern=".*"><p class="field-hint">A Wiki.js API key. Its group needs read:pages, plus manage:pages or read:source to read page content. A saved key is never sent back to the browser, so leaving this blank keeps it.</p></div>"#
    )), "{html}");
    assert!(main.contains("<label>Support Roles</label>"), "{html}");
    assert_eq!(count(main, r#"<div class="chip-list"></div>"#), 2, "{html}");

    let body = "guild=7&idle_enabled=false&idle_after_secs=10&idle_close_enabled=false&idle_close_after_secs=86400";
    let html = app.submit(SUPPORT, body, StatusCode::OK).await.unwrap();
    assert_eq!(settings.support.get(7).await.unwrap().idle_after_secs, 3600);
    assert!(
        main_content(&html).contains(&format!("{}{}", saved(), idle_form("10"))),
        "{html}"
    );
    assert_eq!(count(main_content(&html), "Saved."), 1, "{html}");

    let body = "guild=7&support_channel_id=104&solved_tag_id=600&closed_tag_id=&solved_archive_secs=-5";
    let html = app.submit(SUPPORT, body, StatusCode::OK).await.unwrap();
    let support = settings.support.get(7).await.unwrap();
    assert_eq!(
        (
            support.support_channel_id,
            support.solved_tag_id,
            support.solved_archive_secs
        ),
        (Some(104), Some(600), -1)
    );
    assert!(
        main_content(&html).contains(&format!(
            "{SEGMENTED_SETTINGS}{}{}{}",
            saved(),
            form("support", "7"),
            select("Support Channel", "support_channel_id", "104", TEXT_OPTIONS),
        )),
        "{html}"
    );
    assert!(
        main_content(&html).contains(r#"name="solved_archive_secs" value="-5""#),
        "{html}"
    );

    let body = "guild=7&stale_enabled=true&stale_tag_id=600&stale_after_secs=604800";
    let html = app.submit(SUPPORT, body, StatusCode::OK).await.unwrap();
    let support = settings.support.get(7).await.unwrap();
    assert_eq!((support.stale_enabled, support.stale_tag_id), (true, Some(600)));
    assert!(
        main_content(&html).contains(&format!(
            "{}{}{}",
            saved(),
            form("support", "7"),
            toggle("Mark Quiet Posts Stale", "stale_enabled", true),
        )),
        "{html}"
    );

    let body = "guild=7&suggestions_channel_id=101&review_channel_id=&promote_threshold=10&demote_threshold=12";
    let html = app.submit(SUPPORT, body, StatusCode::OK).await.unwrap();
    let suggestions = settings.suggestions.get(7).await.unwrap();
    assert_eq!(
        (
            suggestions.suggestions_channel_id,
            suggestions.promote_threshold,
            suggestions.demote_threshold
        ),
        (Some(101), 10, 9)
    );
    assert!(
        main_content(&html).contains(&format!(
            "{}{}{}",
            saved(),
            form("support", "7"),
            select(
                "Suggestions Channel",
                "suggestions_channel_id",
                "101",
                TEXT_OPTIONS
            ),
        )),
        "{html}"
    );
    assert!(
        main_content(&html).contains(r#"name="demote_threshold" value="12""#),
        "{html}"
    );

    let body = "guild=7&max_results=99&answer_max_tokens=500&answer_temperature=0.5";
    let html = app.submit(SUPPORT, body, StatusCode::OK).await.unwrap();
    let faq = settings.faq.get(7).await.unwrap();
    assert_eq!(faq.max_results, 25);
    assert!(
        main_content(&html).contains(&format!(
            "{}{}{}",
            saved(),
            form("support", "7"),
            field("Search results to consider", "max_results", "99", "[0-9]*", None),
        )),
        "{html}"
    );

    let body = "guild=7&enabled=true&auto_triage=false&auto_generate=false&wiki_url=ftp%3A%2F%2Fwiki&wiki_locale=en";
    let html =
        app.submit(SUPPORT, body, StatusCode::UNPROCESSABLE_ENTITY).await.unwrap();
    let main = main_content(&html);
    assert!(
        main.contains(&format!(
            "{}{}{}",
            not_saved("the wiki URL must start with http:// or https://"),
            form("support", "7"),
            toggle("Wiki FAQ", "enabled", true),
        )),
        "{html}"
    );
    assert!(main.contains(r#"name="wiki_url" value="ftp://wiki""#), "{html}");
    assert!(!settings.faq.get(7).await.unwrap().enabled);

    let html = app
        .submit(
            SUPPORT,
            "guild=7&wiki_api_key=secret&keep_wiki_api_key=true",
            StatusCode::OK,
        )
        .await
        .unwrap();
    assert!(settings.faq.get(7).await.unwrap().wiki_api_key.is_some());
    let main = main_content(&html);
    assert!(
        main.contains(
            r#"value="" placeholder="A key is saved - leave blank to keep it""#
        ),
        "{html}"
    );
    assert!(main.contains(r#"<option value="true" selected="">Keep</option><option value="false">Remove</option>"#), "{html}");
    assert!(!main.contains("secret"), "{html}");

    let html = app
        .submit(
            SUPPORT,
            "guild=7&wiki_api_key=&keep_wiki_api_key=false",
            StatusCode::OK,
        )
        .await
        .unwrap();
    assert!(settings.faq.get(7).await.unwrap().wiki_api_key.is_none());
    assert!(
        main_content(&html).contains(&format!(
            r#"{}{}<div class="setting-field"><label>Wiki API Key</label><input class="input" type="password" name="wiki_api_key" value="" placeholder="eyJhbGciOiJSUzI1NiIs...""#,
            saved(),
            form("support", "7"),
        )),
        "{html}"
    );
    assert!(
        main_content(&html).contains(
            r#"<input type="hidden" name="keep_wiki_api_key" value="true">"#
        ),
        "{html}"
    );

    let html = app
        .redirected(
            SUPPORT,
            "guild=7&role_id=200",
            "/guild/7/settings/support?role-added=1",
        )
        .await
        .unwrap();
    assert_eq!(support_roles(&app).await.unwrap(), [200]);
    let main = main_content(&html);
    let chip = r#"<div class="chip-list"><form class="chip" method="post" action="/guild/7/settings/support?remove-role" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="role_id" value="200"><span class="chip-label">@Mods</span><button type="submit" class="chip-remove" title="Remove">"#;
    assert!(main.contains(chip), "{html}");
    assert!(main.contains(&format!(
        r#"</div>{}<form class="chip-add" method="post" action="/guild/7/settings/support" data-pending=""><input type="hidden" name="guild" value="7">{}<button type="submit" class="btn btn-ghost">Add role</button></form>"#,
        saved(),
        select("Add a support role", "role_id", "", r#"<option value="201">@Members</option>"#),
    )), "{html}");

    let html = app
        .submit(SUPPORT, "guild=7&role_id=200", StatusCode::UNPROCESSABLE_ENTITY)
        .await
        .unwrap();
    assert!(
        main_content(&html)
            .contains(&not_saved("that role is already a support role")),
        "{html}"
    );

    let html = app
        .redirected(
            &format!("{SUPPORT}?remove-role"),
            "guild=7&role_id=200",
            "/guild/7/settings/support?role-removed=1",
        )
        .await
        .unwrap();
    assert_eq!(support_roles(&app).await.unwrap(), Vec::<u64>::new());
    let main = main_content(&html);
    assert!(!main.contains("@Mods</span>"), "{html}");
    assert!(
        main.contains(&format!(
            r#"<div class="chip-list"></div>{}<form class="chip-add""#,
            saved()
        )),
        "{html}"
    );

    for user in ["501", "502"] {
        let body = format!(
            "guild=7&user_id={user}&link=https%3A%2F%2Fexample.com%2Fd{user}"
        );
        app.redirected(SUPPORT, &body, "/guild/7/settings/support?link-added=1")
            .await
            .unwrap();
    }
    assert_eq!(helper_users(&app).await.unwrap(), [501, 502]);
    let html = app.page(SUPPORT).await.unwrap();
    let main = main_content(&html);
    assert!(main.contains(r#"<span class="chip-label">Helper One → https://example.com/d501</span>"#), "{html}");
    assert!(app.discord.count("GET", "/guilds/7/members/501") > 0);
    assert!(main.contains(r#"<span class="chip-label">unknown (502) → https://example.com/d502</span>"#), "{html}");
    let html = app
        .redirected(
            SUPPORT,
            "guild=7&user_id=502",
            "/guild/7/settings/support?link-removed=1",
        )
        .await
        .unwrap();
    assert_eq!(helper_users(&app).await.unwrap(), [501]);
    let main = main_content(&html);
    assert!(!main.contains("d502"), "{html}");
    assert!(main.contains("https://example.com/d501</span>"), "{html}");
    assert!(
        main.contains(&format!(r#"</div>{}<form class="chip-add""#, saved())),
        "{html}"
    );

    let html = app
        .submit(
            SUPPORT,
            "guild=7&idle_enabled=true&bogus=1",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(
        main_content(&html).contains(&not_saved("unknown field `bogus`")),
        "{html}"
    );

    let html = app.page("/guild/9/settings/support").await.unwrap();
    assert!(main_content(&html).contains(
        r#"<label>Add a support role</label><div class="select"><select class="input" disabled=""><option selected="">(not set)</option></select>"#
    ), "{html}");
    assert!(main_content(&html).contains(
        r#"<input type="hidden" name="role_id" value=""><p class="field-hint field-warning">Couldn't reach Discord; the role list is unavailable. Saving keeps the current value. (error running server function: "#
    ), "{html}");

    let response = app.post(SUPPORT, "guild=7&role_id=200", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn faq_articles_are_listed_saved_and_deleted(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let db = &app.app.db;

    assert!(ticket::FaqArticle::list(db, 7, 10).await.unwrap().is_empty());
    let html = app.page(SUPPORT).await.unwrap();
    let main = main_content(&html);
    let new_form = |open: &str, title: &str, content: &str| {
        format!(
            r#"<details class="setting-field"{open}><summary>New article</summary><form method="post" action="/guild/7/settings/support" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="id" value=""><div class="setting-field"><label>Title</label><input class="input" type="text" name="title" value="{title}" placeholder="Fixing Radarr error 502" pattern=".*"></div><div class="setting-field"><label>Summary</label><input class="input" type="text" name="summary" value="" placeholder="One sentence, shown in search results" pattern=".*"></div><div class="setting-field"><label>Category</label><input class="input" type="text" name="category" value="" placeholder="(not set)" pattern=".*"></div><div class="setting-field"><label>Tags</label><input class="input" type="text" name="tags" value="" placeholder="comma, separated" pattern=".*"><p class="field-hint">Comma separated.</p></div><div class="setting-field"><label>Body (Markdown)</label><textarea class="input" name="content" rows="14">{content}</textarea></div>{SAVE}</form></details>"#
        )
    };
    assert!(main.contains(SEGMENTED_SETTINGS), "{html}");
    assert!(main.contains(&format!(
        r#"<fieldset class="settings-section" hidden=""><p class="page-lead">Articles "/ticket faq ask" and the automated triage search, alongside the wiki. Articles written from solved tickets go live as soon as they are generated, so review them here.</p>{}"#,
        new_form("", "", "")
    )), "{html}");
    assert!(
        html.contains(r#"<p class="page-lead">No FAQ articles yet.</p>"#),
        "{html}"
    );

    let body = "guild=7&id=&title=Fixing+502&summary=&category=&tags=&content=";
    let html =
        app.submit(SUPPORT, body, StatusCode::UNPROCESSABLE_ENTITY).await.unwrap();
    let main = main_content(&html);
    assert!(main.contains(SEGMENTED_FAQ), "{html}");
    assert!(
        main.contains(
            r#"<fieldset class="settings-section"><p class="page-lead">Articles"#
        ),
        "{html}"
    );
    assert!(
        main.contains(&format!(
            "{}{}",
            not_saved("A title and a body are both required."),
            new_form(r#" open="""#, "Fixing 502", "")
        )),
        "{html}"
    );

    // Articles reference `guilds`; any settings save creates the guild's row.
    app.app.settings.support.update(7, |_| {}).await.unwrap();
    let body = "guild=7&id=&title=Fixing+502&summary=Restart+it&category=&tags=Radarr%2C+radarr%2C+HTTP&content=Restart+Radarr.";
    let html = app
        .redirected(SUPPORT, body, "/guild/7/settings/support?article-created=1")
        .await
        .unwrap();
    let articles = ticket::FaqArticle::list(db, 7, 10).await.unwrap();
    let [article] = articles.as_slice() else { panic!("{articles:?}") };
    assert_eq!(article.tags, ["radarr", "http"]);
    let main = main_content(&html);
    assert!(main.contains(SEGMENTED_FAQ), "{html}");
    assert!(
        main.contains(&format!("{}{}", saved(), new_form(r#" open="""#, "", ""))),
        "{html}"
    );
    let id = article.id;
    let row = format!(
        r#"<details class="setting-field"><summary>Fixing 502</summary><p class="field-hint">Updated {}</p><form method="post" action="/guild/7/settings/support" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="id" value="{id}">"#,
        article.updated_at.to_jiff()
    );
    assert!(html.contains(&row), "{html}");
    assert!(html.contains(&format!(
        r#"<textarea class="input" name="content" rows="14">Restart Radarr.</textarea></div>{SAVE}</form><form method="post" action="/guild/7/settings/support" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="id" value="{id}"><div class="form-actions">{}</div></form></details>"#,
        confirm(
            "Delete",
            "This removes the article for everyone, including the wiki copy. It cannot be undone.",
            "Delete article"
        )
    )), "{html}");
    assert!(!html.contains("No FAQ articles yet."), "{html}");

    let body = format!(
        "guild=7&id={id}&title=Fixing+502+again&summary=&category=Media&tags=&content=Restart."
    );
    let html = app.submit(SUPPORT, &body, StatusCode::OK).await.unwrap();
    let updated = ticket::FaqArticle::get(db, 7, id).await.unwrap().unwrap();
    assert_eq!(
        (updated.title.as_str(), updated.category.as_deref()),
        ("Fixing 502 again", Some("Media"))
    );
    assert!(
        main_content(&html).contains(&format!(
            "{}{}",
            saved(),
            new_form("", "", "")
        )),
        "{html}"
    );

    let body = format!(
        "guild=7&id={}&title=Gone&summary=&category=&tags=&content=x",
        id + 1
    );
    let html =
        app.submit(SUPPORT, &body, StatusCode::UNPROCESSABLE_ENTITY).await.unwrap();
    assert!(
        main_content(&html).contains(&not_saved("That article no longer exists.")),
        "{html}"
    );

    let html = app
        .submit(SUPPORT, "guild=7&id=abc", StatusCode::UNPROCESSABLE_ENTITY)
        .await
        .unwrap();
    assert!(
        main_content(&html).contains(&not_saved("That article no longer exists.")),
        "{html}"
    );

    let html = app
        .redirected(
            SUPPORT,
            &format!("guild=7&id={id}"),
            "/guild/7/settings/support?article-deleted=1",
        )
        .await
        .unwrap();
    assert!(ticket::FaqArticle::list(db, 7, 10).await.unwrap().is_empty());
    assert!(main_content(&html).contains(&format!("{}<details", saved())), "{html}");
    assert!(html.contains("No FAQ articles yet."), "{html}");

    let draft = ticket::NewArticle {
        title: "Stuck at 99%",
        summary: "",
        content: "Wait.",
        category: None,
        tags: &[],
    };
    let generated = ticket::FaqArticle::insert_generated(db, 7, 900, 0, draft)
        .await
        .unwrap()
        .unwrap();
    assert!(generated.generated);
    let html = app.page(SUPPORT).await.unwrap();
    assert!(
        html.contains(&format!(
            r#"<summary>Stuck at 99%<span class="chip-label"> generated</span></summary><p class="field-hint">Updated {} • from thread 900</p>"#,
            generated.updated_at.to_jiff()
        )),
        "{html}"
    );

    pool.close().await;
}

#[tokio::test]
async fn provider_sections_report_a_status_they_cannot_load() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(1))
        .connect_lazy("postgres://postgres@127.0.0.1:1/unreachable")
        .unwrap();
    let app = harness(&pool).await.unwrap();

    for (slug, warning, lead) in [
        (
            "youtube",
            "Couldn't load the YouTube connection: error running server function: ",
            "Reload before connecting - connecting while the status is unknown would replace whatever channel is already linked.",
        ),
        (
            "patreon",
            "Couldn't load the Patreon connection: error running server function: ",
            "Reload once Patreon is reachable. Connecting from here while the status is unknown would overwrite whatever campaign is already linked.",
        ),
    ] {
        let html = app.page(&format!("/guild/7/settings/{slug}")).await.unwrap();
        let main = main_content(&html);
        assert!(
            main.contains(&format!(
                r#"<fieldset class="settings-section"><p class="warning">{warning}"#
            )),
            "{html}"
        );
        assert!(
            main.contains(&format!(
                r#"</p><p class="page-lead">{lead}</p></fieldset></div>"#
            )),
            "{html}"
        );
        assert!(!main.contains("connect?guild="), "{html}");
    }
}
