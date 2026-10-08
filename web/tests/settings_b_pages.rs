//! The YouTube and Patreon pages and the Support pages (tickets,
//! suggestions, wiki, roles and links, FAQ articles) through the app router,
//! against Postgres and a stand-in Discord API.
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

/// The result line in the first status region after `marker`.
fn flash_after<'a>(html: &'a str, marker: &str) -> Option<&'a str> {
    let rest = html.split_once(marker)?.1;
    let region =
        between(rest, r#"<div class="flash-region" role="status">"#, "</div>")?;
    between(region, r#"<span class="flash-text">"#, "</span>")
}

/// The result line at the top of the page.
fn top_flash(html: &str) -> Option<&str> {
    flash_after(html, r#"<div class="page-header">"#)
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

fn title(label: &str) -> String {
    format!("<title>{label} - Zayden Dashboard</title>")
}

fn confirm(
    id: &str,
    trigger: &str,
    title: &str,
    prompt: &str,
    confirm: &str,
) -> String {
    format!(
        r#"<div class="confirm" data-confirm=""><button type="submit" class="btn btn-danger" data-confirm-trigger="">{trigger}</button><dialog class="dialog" aria-labelledby="{id}-title" aria-describedby="{id}-desc" data-confirm-dialog=""><div class="dialog-panel"><h2 class="dialog-title" id="{id}-title">{title}</h2><p class="dialog-desc" id="{id}-desc">{prompt}</p><div class="dialog-actions"><button type="button" class="btn btn-secondary" data-dialog-close="" autofocus="">Cancel</button><button type="submit" class="btn btn-danger">{confirm}</button></div></div></dialog></div>"#
    )
}

fn connect_youtube_json() -> youtube::model::OwnChannel {
    youtube::model::OwnChannel {
        id: "UC123".to_owned(),
        title: "Seed Channel".to_owned(),
        uploads_playlist_id: "UU123".to_owned(),
    }
}

const BOT_CANNOT_POST: &str = "Zayden can't post in #rules: it needs View Channel and Send Messages there. Allow them for the bot's role in the channel's permissions, then save again.";

#[sqlx::test(migrations = "../migrations")]
async fn provider_pages_follow_the_connection(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let db = &app.app.db;

    let html = app.page("/guild/7/youtube").await.unwrap();
    assert!(html.contains(&title("YouTube")), "{html}");
    let main = main_content(&html);
    assert!(main.contains(r#"<div class="settings-actions"><span class="lamp-status"><span class="lamp lamp-off" aria-hidden="true"></span><span class="lamp-text">Off</span></span></div>"#), "status text, no switch: {main}");
    assert!(!main.contains(r#"role="switch""#), "{main}");
    assert!(main.contains(r#"<p class="page-lead">No YouTube channel is connected. The channel's owner signs in with Google once to prove it is theirs; Zayden keeps no access to the account afterwards.</p><div class="settings-actions"><a class="btn btn-primary" href="/youtube/connect?guild=7" rel="external">Connect YouTube</a></div>"#), "{main}");
    assert!(!main.contains("<form"), "{main}");

    let html = app.page("/guild/7/youtube?youtube=declined").await.unwrap();
    assert!(
        main_content(&html).contains(r#"<p class="warning" role="status">Authorisation was cancelled, so nothing changed.</p>"#),
        "the callback outcome lands on the feature page: {html}"
    );
    let html = app.page("/guild/7/youtube?youtube=bogus").await.unwrap();
    assert!(
        !main_content(&html).contains(r#"role="status">Authorisation"#),
        "{html}"
    );

    youtube::YoutubeConnection::connect(db, 7, &connect_youtube_json(), 41, "s")
        .await
        .unwrap();
    let html = app.page("/guild/7/youtube?youtube=connected").await.unwrap();
    let main = main_content(&html);
    assert!(main.contains(r#"<p class="success" role="status">YouTube connected. Choose where its uploads should be announced.</p>"#), "{main}");
    assert!(
        main.contains(r#"<p class="page-lead">Connected to Seed Channel.</p>"#),
        "{main}"
    );
    assert!(main.contains(&format!(
        r#"<form id="youtube-disconnect" method="post" action="/guild/7/youtube/disconnect" data-pending=""><input type="hidden" name="guild" value="7">{}</form>"#,
        confirm(
            "youtube-disconnect-confirm",
            "Disconnect",
            "Disconnect YouTube channel Seed Channel?",
            "Zayden stops announcing this channel's uploads. Reconnecting needs the channel owner to sign in with Google again.",
            "Disconnect YouTube"
        )
    )), "{main}");
    assert!(main.contains(r#"<form id="youtube-settings" method="post" action="/guild/7/youtube" data-pending="" data-dirty-guard="">"#), "{main}");
    assert_eq!(selected(main, "youtube-settings-channel-id"), Some(""));
    assert_eq!(duplicate_ids(&html), Vec::<String>::new());

    let html = app
        .submit(
            "/guild/7/youtube",
            "guild=7&channel_id=100",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "youtube-settings"),
        Some(format!("Not saved: {BOT_CANNOT_POST}").as_str())
    );
    assert_eq!(selected(&html, "youtube-settings-channel-id"), Some("100"));
    assert!(youtube::YoutubeAnnounceRow::select(db, 7).await.unwrap().is_none());

    let html = app
        .redirected(
            "/guild/7/youtube",
            "guild=7&channel_id=101",
            "/guild/7/youtube#youtube-settings",
        )
        .await
        .unwrap();
    assert_eq!(
        youtube::YoutubeAnnounceRow::select(db, 7)
            .await
            .unwrap()
            .map(|r| r.channel_id),
        Some(101)
    );
    assert_eq!(
        bar_flash(&html, "youtube-settings"),
        Some("YouTube settings saved.")
    );
    assert_eq!(selected(&html, "youtube-settings-channel-id"), Some("101"));
    assert!(html.contains(r#"<span class="lamp lamp-on" aria-hidden="true"></span><span class="lamp-text">On</span>"#), "{html}");

    let html = app
        .submit(
            "/guild/7/youtube/disconnect",
            "guild=8",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "youtube-disconnect"),
        Some("Not disconnected: invalid value for `guild`")
    );
    assert!(youtube::YoutubeConnection::select(db, 7).await.unwrap().is_some());

    let html = app
        .redirected("/guild/7/youtube/disconnect", "guild=7", "/guild/7/youtube")
        .await
        .unwrap();
    assert!(youtube::YoutubeConnection::select(db, 7).await.unwrap().is_none());
    assert_eq!(
        top_flash(&html),
        Some("YouTube disconnected. Zayden will stop announcing this channel.")
    );
    assert!(main_content(&html).contains("Connect YouTube</a>"), "{html}");

    let html = app.page("/guild/7/patreon").await.unwrap();
    assert!(html.contains(&title("Patreon")), "{html}");
    assert!(main_content(&html).contains(r#"<a class="btn btn-primary" href="/patreon/connect?guild=7" rel="external">Connect Patreon</a>"#), "{html}");

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
    let html = app.page("/guild/7/patreon?patreon=connected").await.unwrap();
    let main = main_content(&html);
    assert!(main.contains(r#"<p class="success" role="status">Patreon connected. Choose where its posts should be announced.</p>"#), "{main}");
    assert!(main.contains(r#"<p class="page-lead">Connected to camp-1, but Patreon has rejected the stored authorisation. Reconnect to resume announcements.</p>"#), "{main}");
    assert!(main.contains(r#"<a class="btn btn-secondary" href="/patreon/connect?guild=7" rel="external">Reconnect Patreon</a>"#), "{main}");
    assert!(main.contains(&confirm(
        "patreon-disconnect-confirm",
        "Disconnect",
        "Disconnect Patreon from camp-1?",
        "Zayden stops announcing this campaign and drops its webhook on the creator's Patreon account. Reconnecting needs the creator to authorise again.",
        "Disconnect Patreon"
    )), "{main}");
    assert_eq!(selected(main, "patreon-settings-public-only"), Some("false"));

    let html = app
        .submit(
            "/guild/7/patreon",
            "guild=7&channel_id=100&public_only=true",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert!(patreon::PatreonAnnounceRow::select(db, 7).await.unwrap().is_none());
    assert_eq!(
        summary(&html, "patreon-settings"),
        Some(format!("Not saved: {BOT_CANNOT_POST}").as_str())
    );
    assert_eq!(selected(&html, "patreon-settings-public-only"), Some("true"));
    assert!(
        !html.contains("patreon=connected"),
        "forms post to the clean page address: {html}"
    );

    let html = app
        .redirected(
            "/guild/7/patreon",
            "guild=7&channel_id=101&public_only=true",
            "/guild/7/patreon#patreon-settings",
        )
        .await
        .unwrap();
    let announce =
        patreon::PatreonAnnounceRow::select(db, 7).await.unwrap().unwrap();
    assert_eq!((announce.channel_id, announce.public_only), (101, true));
    assert_eq!(
        bar_flash(&html, "patreon-settings"),
        Some("Patreon settings saved.")
    );
    assert_eq!(selected(&html, "patreon-settings-public-only"), Some("true"));

    let html = app
        .redirected("/guild/7/patreon/disconnect", "guild=7", "/guild/7/patreon")
        .await
        .unwrap();
    assert!(patreon::PatreonConnection::select(db, 7).await.unwrap().is_none());
    assert_eq!(
        top_flash(&html),
        Some("Patreon disconnected. Zayden will stop announcing this campaign.")
    );

    let response =
        app.post("/guild/7/patreon/disconnect", "guild=7", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));

    let html = app.page("/guild/8/youtube").await.unwrap();
    assert!(
        main_content(&html).contains(
            "You need Manage Server in this server to change its settings."
        ),
        "{html}"
    );
    let html = app
        .submit(
            "/guild/8/youtube",
            "guild=8&channel_id=101",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(summary(&html, "page"), Some("Not saved: forbidden"));

    pool.close().await;
}

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

const TICKETS: &str = "/guild/7/support";

const TICKET_BODY: &str = "guild=7&support_channel_id=104&solved_tag_id=600&closed_tag_id=&solved_archive_secs=-5&idle_enabled=true&idle_after_secs=10&idle_close_enabled=false&idle_close_after_secs=86400&stale_enabled=true&stale_tag_id=600&stale_after_secs=604800";

#[sqlx::test(migrations = "../migrations")]
async fn support_pages_have_their_own_addresses_and_one_form_each(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let settings = &app.app.settings;

    let subnav = |current: &str| {
        let links: String = [
            ("/guild/7/support", "Tickets"),
            ("/guild/7/support/suggestions", "Suggestions"),
            ("/guild/7/support/faq", "FAQ articles"),
            ("/guild/7/support/wiki", "Wiki"),
            ("/guild/7/support/roles", "Roles and links"),
        ]
        .iter()
        .map(|(href, label)| {
            if *label == current {
                format!(r#"<li><a href="{href}" class="btn btn-secondary" aria-current="page">{label}</a></li>"#)
            } else {
                format!(r#"<li><a href="{href}" class="btn btn-ghost">{label}</a></li>"#)
            }
        })
        .collect();
        format!(
            r#"<nav aria-label="Support pages"><ul class="page-subnav">{links}</ul></nav>"#
        )
    };
    for (path, page_title, current) in [
        (TICKETS, "Support", "Tickets"),
        ("/guild/7/support/suggestions", "Suggestions - Support", "Suggestions"),
        ("/guild/7/support/wiki", "Wiki - Support", "Wiki"),
        ("/guild/7/support/roles", "Roles and links - Support", "Roles and links"),
        ("/guild/7/support/faq", "FAQ articles - Support", "FAQ articles"),
    ] {
        let html = app.page(path).await.unwrap();
        assert!(html.contains(&title(page_title)), "{path}: {html}");
        let main = main_content(&html);
        assert!(main.contains(&subnav(current)), "{path}: {main}");
        assert!(main.contains("<h1>Support</h1>"), "{path}");
        assert!(!main.contains(r#"role="tablist""#), "{path}");
        assert_eq!(
            count(
                &html,
                r#"<a href="/guild/7/support" class="nav-link" aria-current="page">"#
            ),
            2,
            "{path}"
        );
        assert_eq!(duplicate_ids(&html), Vec::<String>::new(), "{path}");
    }

    let html = app.page(TICKETS).await.unwrap();
    let main = main_content(&html);
    assert_eq!(
        count(main, "<form"),
        1,
        "tickets, idle and stale share one form: {main}"
    );
    assert_eq!(selected(main, "ticket-settings-solved-tag-id"), Some(""));
    assert!(
        main.contains(r#"<option value="600">#help / Solved</option>"#),
        "{main}"
    );
    assert_eq!(value(main, "ticket-settings-idle-after-secs"), Some("172800"));
    assert!(
        main.contains(
            r#"name="idle_after_secs" value="172800" min="3600" max="2592000""#
        ),
        "{main}"
    );

    let html = app
        .redirected(TICKETS, TICKET_BODY, "/guild/7/support#ticket-settings")
        .await
        .unwrap();
    let support = settings.support.get(7).await.unwrap();
    assert_eq!(
        (
            support.support_channel_id,
            support.solved_tag_id,
            support.solved_archive_secs
        ),
        (Some(104), Some(600), -1)
    );
    assert_eq!((support.idle_enabled, support.idle_after_secs), (true, 3600));
    assert_eq!((support.stale_enabled, support.stale_tag_id), (true, Some(600)));
    assert_eq!(bar_flash(&html, "ticket-settings"), Some("Ticket settings saved."));
    assert_eq!(value(&html, "ticket-settings-idle-after-secs"), Some("3600"));

    let foreign = TICKET_BODY
        .replace("support_channel_id=104", "support_channel_id=999")
        .replace("idle_enabled=true", "idle_enabled=false");
    let html = app
        .submit(TICKETS, &foreign, StatusCode::UNPROCESSABLE_ENTITY)
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "ticket-settings"),
        Some("Not saved: that channel is not in this server")
    );
    assert!(
        settings.support.get(7).await.unwrap().idle_enabled,
        "a rejected channel writes nothing"
    );
    assert_eq!(selected(&html, "ticket-settings-idle-enabled"), Some("false"));

    let html = app
        .submit(
            TICKETS,
            "guild=7&idle_enabled=true&bogus=1",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "ticket-settings"),
        Some("Not saved: unknown field `bogus`")
    );
    let missing = TICKET_BODY.replace("&stale_after_secs=604800", "");
    let html = app
        .submit(TICKETS, &missing, StatusCode::UNPROCESSABLE_ENTITY)
        .await
        .unwrap();
    assert_eq!(
        field_error(&html, "ticket-settings-stale-after-secs"),
        Some("missing field `stale_after_secs`")
    );

    let html = app
        .redirected(
            "/guild/7/support/suggestions",
            "guild=7&suggestions_channel_id=101&review_channel_id=&promote_threshold=10&demote_threshold=12",
            "/guild/7/support/suggestions#suggestion-settings",
        )
        .await
        .unwrap();
    let suggestions = settings.suggestions.get(7).await.unwrap();
    assert_eq!(
        (
            suggestions.suggestions_channel_id,
            suggestions.promote_threshold,
            suggestions.demote_threshold
        ),
        (Some(101), 10, 9)
    );
    assert_eq!(
        bar_flash(&html, "suggestion-settings"),
        Some("Suggestion settings saved.")
    );
    assert_eq!(value(&html, "suggestion-settings-demote-threshold"), Some("9"));

    let wiki = "/guild/7/support/wiki";
    let body = "guild=7&enabled=true&auto_triage=false&auto_generate=true&wiki_url=https%3A%2F%2Fwiki.example.com%2F&wiki_locale=&max_results=99&answer_max_tokens=500&answer_temperature=0.5";
    let html = app
        .redirected(wiki, body, "/guild/7/support/wiki#wiki-settings")
        .await
        .unwrap();
    let faq = settings.faq.get(7).await.unwrap();
    assert!(faq.enabled && faq.auto_generate);
    assert_eq!(faq.wiki_url.as_deref(), Some("https://wiki.example.com"));
    assert_eq!((faq.wiki_locale.as_str(), faq.max_results), ("en", 25));
    assert_eq!(bar_flash(&html, "wiki-settings"), Some("Wiki settings saved."));
    assert_eq!(bar_flash(&html, "wiki-key"), None);

    let body = "guild=7&enabled=false&auto_triage=false&auto_generate=false&wiki_url=ftp%3A%2F%2Fwiki&wiki_locale=en&max_results=3&answer_max_tokens=500&answer_temperature=0.5";
    let html =
        app.submit(wiki, body, StatusCode::UNPROCESSABLE_ENTITY).await.unwrap();
    assert_eq!(
        field_error(&html, "wiki-settings-wiki-url"),
        Some("the wiki URL must start with http:// or https://")
    );
    assert_eq!(value(&html, "wiki-settings-wiki-url"), Some("ftp://wiki"));
    let faq = settings.faq.get(7).await.unwrap();
    assert!(faq.enabled, "the merged form writes nothing when the URL is refused");
    assert_eq!(faq.max_results, 25);

    let html = app
        .redirected(
            "/guild/7/support/wiki/wiki-key",
            "guild=7&wiki_api_key=secret&keep_wiki_api_key=true",
            "/guild/7/support/wiki#wiki-key",
        )
        .await
        .unwrap();
    assert!(settings.faq.get(7).await.unwrap().wiki_api_key.is_some());
    assert_eq!(bar_flash(&html, "wiki-key"), Some("Wiki API key saved."));
    assert!(
        html.contains(r#"placeholder="A key is saved - leave blank to keep it""#),
        "{html}"
    );
    assert_eq!(selected(&html, "wiki-key-keep-wiki-api-key"), Some("true"));
    assert!(!html.contains("secret"), "{html}");

    let html = app
        .redirected(
            "/guild/7/support/wiki/wiki-key",
            "guild=7&wiki_api_key=&keep_wiki_api_key=false",
            "/guild/7/support/wiki#wiki-key",
        )
        .await
        .unwrap();
    assert!(settings.faq.get(7).await.unwrap().wiki_api_key.is_none());
    assert_eq!(bar_flash(&html, "wiki-key"), Some("Wiki API key removed."));
    assert!(
        html.contains(
            r#"<input type="hidden" name="keep_wiki_api_key" value="true">"#
        ),
        "{html}"
    );

    let response = app.post(TICKETS, TICKET_BODY, None).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn support_roles_and_links_are_list_actions_behind_confirms(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let roles = "/guild/7/support/roles";

    let html = app.page(roles).await.unwrap();
    let main = main_content(&html);
    assert!(
        main.contains("Zayden's role must be above the roles it assigns."),
        "{main}"
    );
    assert!(
        main.contains(r#"<p class="page-lead">No support roles yet.</p>"#),
        "{main}"
    );
    assert!(
        main.contains(r#"<p class="page-lead">No helper links yet.</p>"#),
        "{main}"
    );
    assert!(main.contains(r#"<form id="add-role" method="post" action="/guild/7/support/roles/add-role" data-pending="">"#), "{main}");

    let html = app
        .redirected(
            &format!("{roles}/add-role"),
            "guild=7&role_id=200",
            "/guild/7/support/roles#support-roles",
        )
        .await
        .unwrap();
    assert_eq!(support_roles(&app).await.unwrap(), [200]);
    assert_eq!(
        flash_after(&html, r#"id="support-roles""#),
        Some("Support role added.")
    );
    assert_eq!(top_flash(&html), None);
    let main = main_content(&html);
    assert!(
        main.contains(
            r#"<th scope="row" role="rowheader" data-label="Role">@Mods</th>"#
        ),
        "{main}"
    );
    assert!(main.contains(&format!(
        r#"<form method="post" action="/guild/7/support/roles/remove-role" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="role_id" value="200">{}</form>"#,
        confirm(
            "role-200-remove",
            "Remove",
            "Remove support role @Mods?",
            "Its members stop being pinged for new tickets and stop counting as helpers.",
            "Remove role"
        ).replace(r#"class="btn btn-danger" data-confirm-trigger"#, r#"class="btn btn-ghost" data-confirm-trigger"#)
    )), "{main}");
    assert!(
        !element(main, "select", "add-role-role-id")
            .unwrap()
            .contains(r#"value="200""#),
        "a configured role is not offered again"
    );

    let html = app
        .submit(
            &format!("{roles}/add-role"),
            "guild=7&role_id=200",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "add-role"),
        Some("Not added: that role is already a support role")
    );
    assert_eq!(
        field_error(&html, "add-role-role-id"),
        Some("that role is already a support role")
    );

    let html = app
        .redirected(
            &format!("{roles}/remove-role"),
            "guild=7&role_id=200",
            "/guild/7/support/roles#support-roles",
        )
        .await
        .unwrap();
    assert_eq!(support_roles(&app).await.unwrap(), Vec::<u64>::new());
    assert_eq!(
        flash_after(&html, r#"id="support-roles""#),
        Some("Support role removed.")
    );
    assert!(!main_content(&html).contains("@Mods</th>"), "{html}");

    for user in ["501", "502"] {
        let body = format!(
            "guild=7&user_id={user}&link=https%3A%2F%2Fexample.com%2Fd{user}"
        );
        app.redirected(
            &format!("{roles}/add-link"),
            &body,
            "/guild/7/support/roles#helper-links",
        )
        .await
        .unwrap();
    }
    assert_eq!(helper_users(&app).await.unwrap(), [501, 502]);
    let html = app.page(roles).await.unwrap();
    let main = main_content(&html);
    assert!(main.contains(r#"<th scope="row" role="rowheader" data-label="Helper">Helper One</th><td role="cell" data-label="Link"><a href="https://example.com/d501" rel="external">https://example.com/d501</a></td>"#), "{main}");
    assert!(main.contains(r#"data-label="Helper">unknown (502)</th>"#), "{main}");
    assert!(app.discord.count("GET", "/guilds/7/members/501") > 0);
    assert!(main.contains(r#"<h2 class="dialog-title" id="link-501-remove-title">Remove the donation link of Helper One?</h2>"#), "{main}");

    let html = app
        .submit(
            &format!("{roles}/add-link"),
            "guild=7&user_id=503&link=ftp%3A%2F%2Fx",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        field_error(&html, "add-link-link"),
        Some("link must be an http:// or https:// address")
    );
    assert_eq!(value(&html, "add-link-user-id"), Some("503"));

    let html = app
        .redirected(
            &format!("{roles}/remove-link"),
            "guild=7&user_id=502",
            "/guild/7/support/roles#helper-links",
        )
        .await
        .unwrap();
    assert_eq!(helper_users(&app).await.unwrap(), [501]);
    assert_eq!(
        flash_after(&html, r#"id="helper-links""#),
        Some("Helper link removed.")
    );
    assert!(!main_content(&html).contains("d502"), "{html}");

    let html = app.page("/guild/9/support/roles").await.unwrap();
    let main = main_content(&html);
    assert!(main.contains(r#"<select class="input" id="add-role-role-id" aria-describedby="add-role-role-id-help" disabled="">"#), "{main}");
    assert!(
        main.contains("Couldn't reach Discord, so the role list is unavailable."),
        "{main}"
    );

    let response = app
        .post(&format!("{roles}/add-role"), "guild=7&role_id=200", None)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));
    assert_eq!(support_roles(&app).await.unwrap(), Vec::<u64>::new());

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn faq_articles_have_list_new_and_edit_pages(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(&pool).await.unwrap();
    let db = &app.app.db;
    let faq = "/guild/7/support/faq";

    let html = app.page(faq).await.unwrap();
    assert!(
        main_content(&html).contains(r#"<div class="skeleton-list">"#),
        "the list streams in: {html}"
    );
    assert!(html.contains(r#"<div class="empty-state"><h2 class="empty-title">No FAQ articles yet</h2>"#), "{html}");
    assert!(html.contains(r#"<a href="/guild/7/support/faq/new" class="btn btn-primary">New article</a>"#), "{html}");

    let html = app.page(&format!("{faq}/new")).await.unwrap();
    assert!(html.contains(&title("New FAQ article - Support")), "{html}");
    let main = main_content(&html);
    assert!(main.contains(r#"<label class="field-label" for="faq-article-content">Body (Markdown)</label><textarea class="input" id="faq-article-content" name="content" rows="14" required=""#), "the body has a bound label: {main}");
    assert!(main.contains(r#"<form id="faq-article" method="post" action="/guild/7/support/faq/new" data-pending="" data-dirty-guard="">"#), "{main}");
    assert!(
        main.contains(r#"data-pending-label="Saving…">Create article</button>"#),
        "{main}"
    );
    assert_eq!(duplicate_ids(&html), Vec::<String>::new());

    let body = "guild=7&title=Fixing+502&summary=&category=&tags=&content=";
    let html = app
        .submit(&format!("{faq}/new"), body, StatusCode::UNPROCESSABLE_ENTITY)
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "faq-article"),
        Some("Not saved: A title and a body are both required.")
    );
    assert_eq!(
        field_error(&html, "faq-article-content"),
        Some("A title and a body are both required.")
    );
    assert_eq!(value(&html, "faq-article-title"), Some("Fixing 502"));

    app.app.settings.support.update(7, |_| {}).await.unwrap();
    let body = "guild=7&title=Fixing+502&summary=Restart+it&category=&tags=Radarr%2C+radarr%2C+HTTP&content=Restart+Radarr.";
    let response =
        app.post(&format!("{faq}/new"), body, Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let articles = ticket::FaqArticle::list(db, 7, 10).await.unwrap();
    let [article] = articles.as_slice() else { panic!("{articles:?}") };
    assert_eq!(article.tags, ["radarr", "http"]);
    let id = article.id;
    let edit = format!("{faq}/{id}");
    assert_eq!(location(&response), Some(format!("{edit}#faq-article").as_str()));
    let flash = flash_cookie(&response).unwrap();
    let html = body_text(
        app.get(&edit, Some(&format!("{MEMBER}; {flash}"))).await.unwrap(),
    )
    .await
    .unwrap();
    assert!(html.contains(&title("Edit FAQ article - Support")), "{html}");
    assert_eq!(bar_flash(&html, "faq-article"), Some("Article created."));
    assert_eq!(value(&html, "faq-article-tags"), Some("radarr, http"));
    let updated =
        article.updated_at.to_jiff().strftime("%-d %b %Y, %H:%M UTC").to_string();
    assert!(
        html.contains(&format!(r#"<p class="field-hint">Updated {updated}</p>"#)),
        "{html}"
    );
    assert!(html.contains(&confirm(
        &format!("faq-{id}-delete"),
        "Delete article",
        "Delete article “Fixing 502”?",
        "This removes the article for everyone, including the wiki copy. It cannot be undone.",
        "Delete article"
    )), "{html}");

    let html = app.page(faq).await.unwrap();
    assert!(html.contains(&format!(r#"<th scope="row" role="rowheader" data-label="Title"><a href="{edit}">Fixing 502</a></th>"#)), "{html}");
    assert!(html.contains(&format!(r#"<td role="cell" data-label="Updated">{updated}</td><td role="cell" data-label="Source">Written here</td>"#)), "{html}");

    let body = "guild=7&title=Fixing+502+again&summary=&category=Media&tags=&content=Restart.";
    let html =
        app.redirected(&edit, body, &format!("{edit}#faq-article")).await.unwrap();
    let stored = ticket::FaqArticle::get(db, 7, id).await.unwrap().unwrap();
    assert_eq!(
        (stored.title.as_str(), stored.category.as_deref()),
        ("Fixing 502 again", Some("Media"))
    );
    assert_eq!(bar_flash(&html, "faq-article"), Some("Article saved."));

    let missing = format!("{faq}/{}", id + 1);
    let html = app
        .submit(
            &missing,
            "guild=7&title=Gone&summary=&category=&tags=&content=x",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "page"),
        Some("Not saved: That article no longer exists.")
    );
    for path in [missing.as_str(), "/guild/7/support/faq/abc"] {
        let response = app.get(path, Some(MEMBER)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        let html = body_text(response).await.unwrap();
        assert!(html.contains(r#"<h2 class="error-title" id="faq-missing-title">Article not found</h2>"#), "{html}");
        assert!(html.contains(r#"<a href="/guild/7/support/faq" class="btn btn-primary">Back to FAQ articles</a>"#), "{html}");
        assert!(
            html.contains(r#"class="nav-link""#),
            "the guild shell stays: {html}"
        );
    }

    let html = app
        .submit(
            &format!("{edit}/delete"),
            "guild=8",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
        .unwrap();
    assert_eq!(
        summary(&html, "faq-delete"),
        Some("Not deleted: invalid value for `guild`")
    );
    let html =
        app.redirected(&format!("{edit}/delete"), "guild=7", faq).await.unwrap();
    assert!(ticket::FaqArticle::list(db, 7, 10).await.unwrap().is_empty());
    assert_eq!(top_flash(&html), Some("Article deleted."));
    assert!(html.contains("No FAQ articles yet"), "{html}");

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
    let html = app.page(faq).await.unwrap();
    assert!(
        html.contains(
            r#"<td role="cell" data-label="Source">Solved ticket, thread 900</td>"#
        ),
        "{html}"
    );
    let html = app.page(&format!("{faq}/{}", generated.id)).await.unwrap();
    assert!(html.contains(" • written from thread 900</p>"), "{html}");

    pool.close().await;
}

#[tokio::test]
async fn provider_pages_report_a_status_they_cannot_load() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(1))
        .connect_lazy("postgres://postgres@127.0.0.1:1/unreachable")
        .unwrap();
    let app = harness(&pool).await.unwrap();

    for (slug, warning, lead) in [
        (
            "youtube",
            "Couldn't load the YouTube connection: ",
            "Reload before connecting - connecting while the status is unknown would replace whatever channel is already linked.",
        ),
        (
            "patreon",
            "Couldn't load the Patreon connection: ",
            "Reload once Patreon is reachable. Connecting from here while the status is unknown would overwrite whatever campaign is already linked.",
        ),
    ] {
        let html = app.page(&format!("/guild/7/{slug}")).await.unwrap();
        let main = main_content(&html);
        assert!(
            main.contains(&format!(r#"<p class="warning" role="alert">{warning}"#)),
            "{html}"
        );
        assert!(
            main.contains(&format!(
                r#"</p><p class="page-lead">{lead}</p></section>"#
            )),
            "{html}"
        );
        assert!(!main.contains("error running server function"), "{html}");
        assert!(!main.contains("connect?guild="), "{html}");
    }
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
