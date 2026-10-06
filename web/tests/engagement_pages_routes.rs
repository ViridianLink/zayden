//! The levels, reaction-role and greetings pages through the app router,
//! against Postgres and a stand-in Discord API.
//!
//! Sessions and Discord guild lists come from seeded caches. The bot's Discord
//! client is pointed at a local server that answers the endpoints each case
//! registers and records every request, so a case can also assert what was
//! never sent. Each `#[sqlx::test]` builds one full app state and runs its
//! scenarios in sequence, with a database pool of three connections.

use std::error::Error;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread;
use std::time::Duration;

use http_body_util::BodyExt;
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use topcoat::Result as ViewResult;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::context::Cx;
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{
    Body,
    Router,
    RouterBuilder,
    StatusCode,
    header,
    page,
    path_param,
};
use topcoat::view::{View, view};
use twilight_model::channel::ChannelType;
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use url::form_urlencoded;
use web::auth::ChannelInfo;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::engagement::pages::greetings::channel_section;
use web::engagement::pages::{GREETINGS_TITLE, LEVELS_TITLE, REACTION_ROLES_TITLE};
use web::state::{DiscordState, SessionIdentity, WebState};
use zayden_app::config::BotConfig;
use zayden_app::entitlement::{EntitlementScope, Tier};
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const ADMIN: &str = "session=admin-token";
const MEMBER: &str = "session=member-token";
const OPERATOR: &str = "session=operator-token";
const APP_ID: u64 = 123_456_789;
const AVATAR: &str = "abcdef0123456789abcdef0123456789";
const TEST_POOL_CONNECTIONS: u32 = 3;
const FAILED_SAVE: &str = r#"<div class="alert error" role="alert"><span>Failed to save: error running server function: "#;
const SAVED: &str =
    r#"<div class="alert success" role="status"><span>Saved.</span>"#;
const SAVED_ALERT: &str = r#"<div class="alert success" role="status"><span>Saved.</span><button type="button" class="alert-dismiss" aria-label="Dismiss"><svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg></button></div>"#;

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

    fn get(&self, path: &str, body: &str) {
        self.reply("GET", path, 200, body);
    }

    fn hits(&self, method: &str, prefix: &str) -> Vec<Hit> {
        locked(&self.hits)
            .iter()
            .filter(|h| h.method == method && h.path.starts_with(prefix))
            .cloned()
            .collect()
    }

    fn count(&self, method: &str, prefix: &str) -> usize {
        self.hits(method, prefix).len()
    }
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn user_json(
    id: u64,
    name: &str,
    global: Option<&str>,
    avatar: Option<&str>,
) -> String {
    let global = global.map_or_else(|| "null".to_owned(), |g| format!("\"{g}\""));
    let avatar = avatar.map_or_else(|| "null".to_owned(), |a| format!("\"{a}\""));
    format!(
        r#"{{"id":"{id}","username":"{name}","global_name":{global},"avatar":{avatar},"discriminator":"0","bot":false}}"#
    )
}

fn guild_json(id: u64, owner: u64) -> String {
    format!(
        r#"{{"afk_channel_id":null,"afk_timeout":300,"application_id":null,"banner":null,"default_message_notifications":0,"description":null,"discovery_splash":null,"emojis":[],"explicit_content_filter":0,"features":[],"icon":null,"id":"{id}","large":false,"mfa_level":0,"name":"Guild {id}","nsfw_level":0,"owner_id":"{owner}","preferred_locale":"en-US","premium_progress_bar_enabled":false,"public_updates_channel_id":null,"roles":[],"rules_channel_id":null,"splash":null,"system_channel_flags":0,"system_channel_id":null,"vanity_url_code":null,"verification_level":0}}"#
    )
}

fn channel_json(id: u64, name: &str, kind: u8, position: u32) -> String {
    format!(r#"{{"id":"{id}","type":{kind},"name":"{name}","position":{position}}}"#)
}

fn role_json(id: u64, name: &str, position: i64) -> String {
    format!(
        r#"{{"color":0,"colors":{{"primary_color":0}},"hoist":false,"id":"{id}","managed":false,"mentionable":false,"name":"{name}","permissions":"0","position":{position},"flags":0}}"#
    )
}

fn message_json(id: u64, channel: u64) -> String {
    format!(
        r#"{{"attachments":[],"author":{},"channel_id":"{channel}","content":"","edited_timestamp":null,"embeds":[],"id":"{id}","mention_everyone":false,"mention_roles":[],"mentions":[],"pinned":false,"timestamp":"2020-01-01T00:00:00.000000+00:00","tts":false,"type":0}}"#,
        user_json(APP_ID, "zayden", None, None)
    )
}

fn command_json(id: u64, name: &str) -> String {
    format!(
        r#"{{"id":"{id}","application_id":"{APP_ID}","name":"{name}","description":"d","type":1,"version":"1","options":[]}}"#
    )
}

fn permissions_json(
    guild: u64,
    command: u64,
    entries: &[(u64, u8, bool)],
) -> String {
    let entries = entries
        .iter()
        .map(|(id, kind, allow)| {
            format!(r#"{{"id":"{id}","type":{kind},"permission":{allow}}}"#)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"application_id":"{APP_ID}","guild_id":"{guild}","id":"{command}","permissions":[{entries}]}}"#
    )
}

fn list(items: &[String]) -> String {
    format!("[{}]", items.join(","))
}

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-engagement-pages-assets-{}", std::process::id()));
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
        zayden_id: APP_ID,
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

fn guild(id: u64, name: &str, permissions: Permissions) -> CurrentUserGuild {
    CurrentUserGuild {
        id: Id::new(id),
        name: name.to_owned(),
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
    pool: PgPool,
    discord: Discord,
    app: Arc<ZaydenAppState>,
}

struct Answer {
    status: StatusCode,
    html: String,
    location: Option<String>,
}

fn web_state(pool: PgPool) -> TestResult<(WebState, Discord, Arc<ZaydenAppState>)> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let discord = Discord::start()?;
    let config = config();
    let app = Arc::new(ZaydenAppState::new(pool, &config));
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

    Ok((state, discord, app))
}

fn harness_without_database(pool: PgPool) -> TestResult<Harness> {
    let (state, discord, app) = web_state(pool.clone())?;
    let base = Router::builder()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .app_context(state);

    Ok(Harness { router: web::router(base), pool, discord, app })
}

async fn harness(
    pool: PgPool,
    extra: impl FnOnce(RouterBuilder) -> RouterBuilder,
) -> TestResult<Harness> {
    let (state, discord, app) = web_state(pool.clone())?;

    for (token, user_id) in
        [("admin-token", 41), ("member-token", 43), ("operator-token", 44)]
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
                guild(7, "Guild 7", Permissions::MANAGE_GUILD),
                guild(11, "Guild 11", Permissions::MANAGE_GUILD),
                guild(12, "Guild 12", Permissions::ADMINISTRATOR),
            ]),
        )
        .await;
    state
        .discord
        .user_guilds
        .insert(43, Arc::from([guild(7, "Guild 7", Permissions::SEND_MESSAGES)]))
        .await;
    state.discord.user_guilds.insert(44, Arc::from([])).await;
    sqlx::query!(
        "INSERT INTO web_user_roles (discord_user_id, role) VALUES ($1, $2)",
        44_i64,
        "operator"
    )
    .execute(&pool)
    .await?;

    let base = Router::builder()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .app_context(state);

    Ok(Harness { router: web::router(extra(base)), pool, discord, app })
}

impl Harness {
    async fn send(&self, request: Request<Body>) -> TestResult<Answer> {
        let response = self.router.handle(request).await;
        let status = response.status();
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        Ok(Answer { status, html: body_text(response).await?, location })
    }

    async fn get(&self, path: &str, cookie: Option<&str>) -> TestResult<Answer> {
        let mut request = Request::get(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        self.send(request.body(Body::empty())?).await
    }

    async fn post(
        &self,
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
        self.send(request.body(Body::from(body))?).await
    }

    async fn page(&self, path: &str) -> TestResult<String> {
        let reply = self.get(path, Some(ADMIN)).await?;
        if reply.status != StatusCode::OK {
            return Err(format!("{path} answered {}", reply.status).into());
        }
        Ok(reply.html)
    }

    async fn seed_guild(&self, id: i64) -> TestResult {
        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT DO NOTHING",
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_user(&self, id: i64) -> TestResult {
        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            id,
            format!("user{id}")
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_mapping(
        &self,
        guild_id: i64,
        channel: i64,
        message: i64,
        role: i64,
        emoji: &str,
    ) -> TestResult {
        self.seed_guild(guild_id).await?;
        sqlx::query!(
            "INSERT INTO reaction_roles (guild_id, channel_id, message_id, role_id, emoji) \
             VALUES ($1, $2, $3, $4, $5)",
            guild_id,
            channel,
            message,
            role,
            emoji
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mappings(&self, guild_id: i64) -> TestResult<Vec<String>> {
        Ok(sqlx::query!(
            "SELECT channel_id, message_id, role_id, emoji FROM reaction_roles \
             WHERE guild_id = $1 ORDER BY id",
            guild_id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| {
            format!("{}/{}/{}/{}", r.channel_id, r.message_id, r.role_id, r.emoji)
        })
        .collect())
    }

    async fn images(&self, guild_id: i64, kind: &str) -> TestResult<Vec<String>> {
        Ok(sqlx::query_scalar!(
            "SELECT url FROM greeting_images WHERE guild_id = $1 AND kind = $2 ORDER BY id",
            guild_id,
            kind
        )
        .fetch_all(&self.pool)
        .await?)
    }

    fn guild_directory(&self, guild: u64) {
        self.discord.get(
            &format!("/guilds/{guild}/channels"),
            &list(&[
                channel_json(21, "chat", 0, 2),
                channel_json(20, "rules", 0, 1),
                channel_json(22, "bots", 0, 3),
                channel_json(23, "Lounge", 4, 0),
            ]),
        );
        self.discord.get(
            &format!("/guilds/{guild}/roles"),
            &list(&[
                role_json(guild, "@everyone", 0),
                role_json(30, "Member", 1),
                role_json(31, "Mod", 2),
            ]),
        );
    }

    fn mock_free_guild(&self, guild: u64) {
        self.discord.get(&format!("/guilds/{guild}"), &guild_json(guild, 1));
        self.discord.get(&format!("/applications/{APP_ID}/commands"), "[]");
        self.discord
            .get(&format!("/applications/{APP_ID}/guilds/{guild}/commands"), "[]");
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

fn title(text: &str) -> String {
    format!("<title>{text}</title>")
}

fn login_redirect(reply: &Answer) -> bool {
    reply.status == StatusCode::SEE_OTHER
        && reply.location.as_deref() == Some("/login")
}

/// The attribute value after `marker`, up to the next quote.
fn attribute_after<'a>(html: &'a str, marker: &str) -> Option<&'a str> {
    let (_, tail) = html.split_once(marker)?;
    tail.split_once('"').map(|(value, _)| value)
}

fn row(
    rank: u32,
    avatar: &str,
    name: &str,
    level: i32,
    xp: i32,
    messages: i64,
) -> String {
    format!(
        r#"<div class="lb-row"><span class="lb-rank">{rank}</span><span class="lb-user">{avatar}<span class="lb-name">{name}</span></span><span class="lb-num">{level}</span><span class="lb-num">{xp}</span><span class="lb-num">{messages}</span></div>"#
    )
}

const PLACEHOLDER: &str = r#"<span class="lb-avatar placeholder"></span>"#;
const HEAD_ROW: &str = r#"<div class="lb-row lb-head"><span class="lb-rank">#</span><span class="lb-user">Member</span><span class="lb-num">Level</span><span class="lb-num">XP</span><span class="lb-num">Messages</span></div>"#;

fn avatar(id: u64, hash: &str) -> String {
    format!(
        r#"<img class="lb-avatar" src="https://cdn.discordapp.com/avatars/{id}/{hash}.png" alt="">"#
    )
}

fn scope(guild: u64, global: bool) -> String {
    let (server, world) =
        if global { ("seg", "seg active") } else { ("seg active", "seg") };
    let global_href = format!("/guild/{guild}/levels?scope=global");
    format!(
        r#"<div class="segmented" role="tablist"><a class="{server}" href="/guild/{guild}/levels">This server</a><a class="{world}" href="{global_href}">Global</a></div>"#
    )
}

fn previous_link(href: &str) -> String {
    format!(r#"<a class="btn btn-secondary" href="{href}">Previous</a>"#)
}

fn next_link(href: &str) -> String {
    format!(r#"<a class="btn btn-secondary" href="{href}">Next</a>"#)
}

const PREVIOUS_OFF: &str = r#"<button type="button" class="btn btn-secondary" disabled="">Previous</button>"#;
const NEXT_OFF: &str =
    r#"<button type="button" class="btn btn-secondary" disabled="">Next</button>"#;

fn pager(previous: &str, page: u32, next: &str) -> String {
    format!(
        r#"<div class="pager">{previous}<span class="pager-page">Page {page}</span>{next}</div>"#
    )
}

#[sqlx::test(migrations = "../migrations")]
async fn the_levels_page_pages_the_board_by_links_and_names_members_from_discord(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();

    let empty = app.page("/guild/11/levels").await.unwrap();
    assert!(empty.contains(&title(LEVELS_TITLE)), "{empty}");
    assert!(empty.contains(r"<h1>Levels</h1>"), "{empty}");
    assert!(
        empty.contains(
            r#"<p class="page-lead">Message-XP rankings. Switch between this server and the global board.</p>"#
        ),
        "{empty}"
    );
    assert!(empty.contains(&scope(11, false)), "{empty}");
    assert!(
        empty.contains(
            r#"<div class="empty">No one has chatted here yet - the board fills as members talk.</div>"#
        ),
        "{empty}"
    );
    assert!(!empty.contains("leaderboard"), "{empty}");
    assert!(!empty.contains(r#"class="pager""#), "{empty}");
    let empty = app.page("/guild/11/levels?scope=global").await.unwrap();
    assert!(empty.contains(&scope(11, true)), "{empty}");
    assert!(
        empty.contains(
            r#"<div class="empty">No one has earned global XP yet.</div>"#
        ),
        "{empty}"
    );

    app.seed_guild(7).await.unwrap();
    app.seed_guild(9).await.unwrap();
    for i in 0..13_i64 {
        app.seed_user(1000 + i).await.unwrap();
    }
    for i in 0..12_i64 {
        sqlx::query!(
            "INSERT INTO guild_levels (guild_id, user_id, xp, level, message_count) \
             VALUES (7, $1, $2, $3, $4)",
            1000 + i,
            100 - i32::try_from(i).unwrap(),
            20 - i32::try_from(i).unwrap(),
            50 + i
        )
        .execute(&app.pool)
        .await
        .unwrap();
    }
    sqlx::query!(
        "INSERT INTO guild_levels (guild_id, user_id, xp, level, message_count) \
         VALUES (9, 1012, 5000, 90, 1)"
    )
    .execute(&app.pool)
    .await
    .unwrap();
    for (user, level, xp, messages) in [(1001_i64, 40, 10, 7), (1000, 40, 30, 8)] {
        sqlx::query!(
            "INSERT INTO levels (user_id, xp, level, message_count) VALUES ($1, $2, $3, $4)",
            user,
            xp,
            level,
            messages
        )
        .execute(&app.pool)
        .await
        .unwrap();
    }
    app.discord.get(
        "/users/1000",
        &user_json(1000, "alice", Some("Alice A"), Some(AVATAR)),
    );
    app.discord.get("/users/1001", &user_json(1001, "bob", None, None));
    app.discord.reply("GET", "/users/1002", 500, "{}");

    let first = app.page("/guild/7/levels").await.unwrap();
    assert!(first.contains(&title(LEVELS_TITLE)), "{first}");
    assert!(first.contains(&scope(7, false)), "{first}");
    assert!(
        first.contains(&format!(r#"<div class="leaderboard">{HEAD_ROW}"#)),
        "{first}"
    );
    assert_eq!(count(&first, r#"<div class="lb-row">"#), 10, "{first}");
    assert!(
        first.contains(&row(1, &avatar(1000, AVATAR), "Alice A", 20, 100, 50)),
        "{first}"
    );
    assert!(first.contains(&row(2, PLACEHOLDER, "bob", 19, 99, 51)), "{first}");
    assert!(
        first.contains(&row(3, PLACEHOLDER, "User 1002", 18, 98, 52)),
        "{first}"
    );
    assert!(
        first.contains(&row(10, PLACEHOLDER, "User 1009", 11, 91, 59)),
        "{first}"
    );
    assert!(!first.contains("User 1010"), "{first}");
    assert!(
        first.contains(&pager(
            PREVIOUS_OFF,
            1,
            &next_link("/guild/7/levels?page=2")
        )),
        "{first}"
    );
    assert_eq!(app.discord.count("GET", "/users/"), 10);
    let skeleton = format!(
        r#"<div class="skeleton-list">{}</div>"#,
        r#"<div class="skeleton-row" aria-hidden="true"></div>"#.repeat(10)
    );
    assert!(first.contains(&skeleton), "{first}");

    let escaped =
        app.get("/guild/7%3Fscope=global/levels", Some(ADMIN)).await.unwrap();
    assert!(
        escaped.html.contains(
            r#"<a class="seg active" href="/guild/7%3Fscope%3Dglobal/levels">"#
        ),
        "{}",
        escaped.html
    );
    assert!(
        escaped
            .html
            .contains(r#"href="/guild/7%3Fscope%3Dglobal/levels?scope=global">"#),
        "{}",
        escaped.html
    );

    let second = app.page("/guild/7/levels?page=2").await.unwrap();
    assert_eq!(count(&second, r#"<div class="lb-row">"#), 2, "{second}");
    assert!(
        second.contains(&row(11, PLACEHOLDER, "User 1010", 10, 90, 60)),
        "{second}"
    );
    assert!(
        second.contains(&row(12, PLACEHOLDER, "User 1011", 9, 89, 61)),
        "{second}"
    );
    assert!(
        second.contains(&pager(&previous_link("/guild/7/levels"), 2, NEXT_OFF)),
        "{second}"
    );

    let past = app.page("/guild/7/levels?page=3").await.unwrap();
    assert!(
        past.contains(r#"<div class="empty">No more entries on this page.</div>"#),
        "{past}"
    );
    assert!(!past.contains("leaderboard"), "{past}");
    assert!(
        past.contains(&pager(&previous_link("/guild/7/levels?page=2"), 3, NEXT_OFF)),
        "{past}"
    );

    let world = app.page("/guild/7/levels?scope=global").await.unwrap();
    assert!(world.contains(&scope(7, true)), "{world}");
    assert!(
        world.contains(&row(1, &avatar(1000, AVATAR), "Alice A", 40, 30, 8)),
        "{world}"
    );
    assert!(world.contains(&row(2, PLACEHOLDER, "bob", 40, 10, 7)), "{world}");
    assert_eq!(count(&world, r#"<div class="lb-row">"#), 2, "{world}");
    assert!(!world.contains(r#"class="pager""#), "{world}");

    let lenient = app.page("/guild/7/levels?scope=weird&page=-4").await.unwrap();
    assert!(lenient.contains(&scope(7, false)), "{lenient}");
    assert!(lenient.contains(&row(
        1,
        &avatar(1000, AVATAR),
        "Alice A",
        20,
        100,
        50
    )));
    let paged = app.page("/guild/7/levels?scope=global&page=2").await.unwrap();
    assert!(
        paged.contains(r#"<div class="empty">No more entries on this page.</div>"#),
        "{paged}"
    );
    assert!(
        paged.contains(&pager(
            &previous_link("/guild/7/levels?scope=global"),
            2,
            NEXT_OFF
        )),
        "{paged}"
    );

    let signed_out = app.get("/guild/7/levels", None).await.unwrap();
    assert!(login_redirect(&signed_out));

    let member = app.get("/guild/7/levels", Some(MEMBER)).await.unwrap();
    assert_eq!(member.status, StatusCode::OK);
    assert!(
        member.html.contains(
            r#"<p class="error">Failed to load leaderboard: error running server function: forbidden</p>"#
        ),
        "{}",
        member.html
    );
    assert!(!member.html.contains("leaderboard\""), "{}", member.html);

    let outsider = app.get("/guild/9/levels", Some(ADMIN)).await.unwrap();
    assert!(outsider.html.contains(
        "Failed to load leaderboard: error running server function: forbidden"
    ));
    let malformed = app.get("/guild/abc/levels", Some(ADMIN)).await.unwrap();
    assert!(malformed.html.contains(
        "Failed to load leaderboard: error running server function: invalid guild id"
    ));

    app.pool.close().await;
}

const RR_LEAD: &str = r#"<p class="page-lead">Every message → emoji → role mapping in this server, in one place."#;

fn rr_row(
    channel: &str,
    emoji: &str,
    role: &str,
    link: &str,
    remove: &str,
    fields: [&str; 3],
) -> String {
    let [channel_id, message_id, emoji_value] = fields;
    format!(
        r#"<div class="rr-row"><span class="rr-channel">{channel}</span><span class="rr-cell">{emoji}</span><span class="rr-role">{role}</span><a class="rr-link" href="{link}" rel="external noreferrer" target="_blank">Message<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg></a><form class="rr-remove" method="post" action="{remove}" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="channel_id" value="{channel_id}"><input type="hidden" name="message_id" value="{message_id}"><input type="hidden" name="emoji" value="{emoji_value}"><details class="confirm"><summary class="btn btn-ghost"><span class="confirm-label">Remove</span><span class="confirm-cancel">Cancel</span></summary><div class="confirm-panel"><p class="confirm-prompt">Reactions already on the message stay, but they stop granting the role.</p><button type="submit" class="btn btn-danger">Remove mapping</button></div></details></form></div>"#
    )
}

const RR_ADD: &str = "/guild/7/reaction-roles?action=add";
const RR_REMOVE: &str = "/guild/7/reaction-roles?action=remove";

#[sqlx::test(migrations = "../migrations")]
async fn the_reaction_role_page_lists_mappings_and_adds_and_removes_them(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();
    app.guild_directory(7);

    let empty = app.page("/guild/7/reaction-roles").await.unwrap();
    assert!(empty.contains(&title(REACTION_ROLES_TITLE)), "{empty}");
    assert!(empty.contains("<h1>Reaction Roles</h1>"), "{empty}");
    assert!(empty.contains(RR_LEAD), "{empty}");
    assert!(
        empty.contains(
            r#"<div class="empty">No reaction roles yet - add one below and Zayden will seed the reaction for members to click.</div>"#
        ),
        "{empty}"
    );
    assert!(!empty.contains("rr-table"), "{empty}");
    assert!(empty.contains(r#"<legend><svg class="icon""#), "{empty}");
    assert!(empty.contains("Add a mapping</legend>"), "{empty}");
    assert!(
        empty.contains(&format!(
            r#"<form method="post" action="{RR_ADD}" data-pending=""><input type="hidden" name="guild" value="7">"#
        )),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<select class="input" name="channel_id"><option value="" selected="">(not set)</option><option value="20"># rules</option><option value="21"># chat</option><option value="22"># bots</option></select>"#
        ),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<label>Message ID (blank posts a new panel)</label><input class="input" type="text" name="message_id" value="" placeholder="(not set)" pattern="[0-9]*">"#
        ),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<label>Emoji</label><input class="input" type="text" name="emoji" placeholder="✅ or <:name:id>">"#
        ),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<select class="input" name="role_id"><option value="" selected="">(not set)</option><option value="31">@Mod</option><option value="30">@Member</option></select>"#
        ),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<div class="form-actions"><button type="submit" class="btn btn-primary">Add mapping</button></div></form><p class="page-lead">Leave the message ID blank and Zayden posts a new panel message in the chosen channel."#
        ),
        "{empty}"
    );
    assert!(!empty.contains(r#"class="alert"#), "{empty}");

    app.discord.reply("POST", "/channels/20/messages", 200, &message_json(555, 20));
    app.discord.get("/channels/20/messages/555", &message_json(555, 20));
    app.discord.reply(
        "PUT",
        "/channels/20/messages/555/reactions/%E2%9C%85/@me",
        204,
        "",
    );
    app.discord.reply("PUT", "/channels/20/messages/555/reactions/a:5/@me", 204, "");
    app.discord.reply(
        "DELETE",
        "/channels/20/messages/555/reactions/%E2%9C%85",
        204,
        "",
    );

    let add = |message: &'static str, role: &'static str, emoji: &'static str| {
        let app = &app;
        async move {
            app.post(
                RR_ADD,
                &[
                    ("guild", "7"),
                    ("channel_id", "20"),
                    ("message_id", message),
                    ("role_id", role),
                    ("emoji", emoji),
                ],
                Some(ADMIN),
            )
            .await
            .unwrap()
        }
    };

    let unseeded = add("", "30", "\u{2705}").await;
    assert_eq!(unseeded.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(unseeded.html.contains(FAILED_SAVE), "{}", unseeded.html);
    assert!(
        unseeded.html.contains("reaction_roles_guild_id_fkey"),
        "{}",
        unseeded.html
    );
    assert!(unseeded.html.contains(&title(REACTION_ROLES_TITLE)));
    assert!(unseeded.html.contains(r#"<div class="empty">No reaction roles yet"#));
    assert_eq!(app.mappings(7).await.unwrap().len(), 0);

    app.seed_guild(7).await.unwrap();
    let created = add("", "30", "\u{2705}").await;
    assert_eq!(created.status, StatusCode::SEE_OTHER);
    assert_eq!(created.location.as_deref(), Some("/guild/7/reaction-roles?added=1"));
    assert_eq!(app.mappings(7).await.unwrap(), ["20/555/30/\u{2705}"]);
    assert_eq!(app.discord.count("POST", "/channels/20/messages"), 2);
    let saved =
        app.get("/guild/7/reaction-roles?added=1", Some(ADMIN)).await.unwrap();
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(app.discord.count("POST", "/channels/20/messages"), 2);
    assert_eq!(count(&saved.html, SAVED), 1, "{}", saved.html);
    assert!(
        saved.html.contains(r#"<fieldset class="settings-section"><legend>"#),
        "{}",
        saved.html
    );
    let alert_at = saved.html.find(SAVED).expect("saved alert");
    let form_at = saved.html.find(&format!(r#"action="{RR_ADD}""#)).unwrap();
    let table_at = saved.html.find("rr-table").unwrap();
    assert!(table_at < alert_at && alert_at < form_at, "{}", saved.html);
    assert!(
        saved.html.contains(&rr_row(
            "#rules",
            r#"<span class="rr-emoji">✅</span>"#,
            "@Member",
            "https://discord.com/channels/7/20/555",
            RR_REMOVE,
            ["20", "555", "\u{2705}"],
        )),
        "{}",
        saved.html
    );

    let duplicate = add("555", "31", "\u{2705}").await;
    assert_eq!(duplicate.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        duplicate.html.contains(&format!(
            "{FAILED_SAVE}that emoji is already mapped on that message</span>"
        )),
        "{}",
        duplicate.html
    );
    assert_eq!(count(&duplicate.html, r#"class="rr-row""#), 1, "{}", duplicate.html);

    let created = add("555", "31", "<:a:5>").await;
    assert_eq!(created.status, StatusCode::SEE_OTHER);
    let custom =
        app.get("/guild/7/reaction-roles?added=1", Some(ADMIN)).await.unwrap();
    assert!(
        custom.html.contains(
            r#"<span class="rr-cell"><img class="rr-emoji-img" src="https://cdn.discordapp.com/emojis/5.png?size=32" alt=""></span><span class="rr-role">@Mod</span>"#
        ),
        "{}",
        custom.html
    );
    assert_eq!(count(&custom.html, r#"class="rr-row""#), 2, "{}", custom.html);

    app.seed_mapping(7, 99, 1, 98, "\u{1f3ae}").await.unwrap();
    let listed = app.page("/guild/7/reaction-roles").await.unwrap();
    assert_eq!(count(&listed, r#"class="rr-row""#), 3, "{listed}");
    assert!(
        listed.contains(r#"<span class="rr-channel">#unknown (99)</span>"#),
        "{listed}"
    );
    assert!(
        listed.contains(r#"<span class="rr-role">@unknown (98)</span>"#),
        "{listed}"
    );
    assert!(
        listed.contains(
            r#"<div class="rr-row rr-head"><span>Channel</span><span>Emoji</span><span>Role</span><span></span><span></span></div>"#
        ),
        "{listed}"
    );
    assert!(!listed.contains(r#"class="alert"#), "{listed}");

    let remove =
        |channel: &'static str, message: &'static str, emoji: &'static str| {
            let app = &app;
            async move {
                app.post(
                    RR_REMOVE,
                    &[
                        ("guild", "7"),
                        ("channel_id", channel),
                        ("message_id", message),
                        ("emoji", emoji),
                    ],
                    Some(ADMIN),
                )
                .await
                .unwrap()
            }
        };
    let done = remove("20", "555", "\u{2705}").await;
    assert_eq!(done.status, StatusCode::SEE_OTHER);
    assert_eq!(done.location.as_deref(), Some("/guild/7/reaction-roles?removed=1"));
    assert_eq!(app.mappings(7).await.unwrap().len(), 2);
    let removed =
        app.get("/guild/7/reaction-roles?removed=1", Some(ADMIN)).await.unwrap();
    assert_eq!(removed.status, StatusCode::OK);
    let alert_at = removed.html.find(SAVED).expect("saved alert");
    assert!(alert_at < removed.html.find("rr-table").unwrap(), "{}", removed.html);
    assert_eq!(count(&removed.html, SAVED), 1, "{}", removed.html);
    assert_eq!(app.discord.count("DELETE", "/channels/20/messages/555/"), 1);

    for query in ["added=2", "added=", "removed=yes", "other=1", "added=1&removed=1"]
    {
        let reply = app
            .get(&format!("/guild/7/reaction-roles?{query}"), Some(ADMIN))
            .await
            .unwrap();
        assert_eq!(reply.status, StatusCode::OK, "{query}");
        let expected = usize::from(query == "added=1&removed=1");
        assert_eq!(count(&reply.html, SAVED), expected, "{query}");
    }
    let flagged = app
        .get("/guild/7/reaction-roles?added=1&removed=1", Some(ADMIN))
        .await
        .unwrap();
    assert!(
        flagged.html.find(SAVED).unwrap()
            > flagged.html.find("Add a mapping").unwrap(),
        "{}",
        flagged.html
    );

    let invalid = remove("x", "555", "\u{2705}").await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        invalid.html.contains(&format!("{FAILED_SAVE}invalid channel</span>")),
        "{}",
        invalid.html
    );
    assert_eq!(count(&invalid.html, "alert error"), 1, "{}", invalid.html);

    let invalid = add("", "", "\u{2705}").await;
    assert!(
        invalid.html.contains(&format!("{FAILED_SAVE}invalid role</span>")),
        "{}",
        invalid.html
    );

    let writes = |app: &Harness| {
        ["POST", "PUT", "DELETE"].map(|method| app.discord.count(method, "/"))
    };
    let sent = writes(&app);
    let missing = app
        .post(
            RR_ADD,
            &[
                ("guild", "7"),
                ("channel_id", "20"),
                ("message_id", ""),
                ("emoji", "x"),
            ],
            Some(ADMIN),
        )
        .await
        .unwrap();
    assert_eq!(missing.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        missing
            .html
            .contains(&format!("{FAILED_SAVE}missing field `role_id`</span>")),
        "{}",
        missing.html
    );
    let unknown = app
        .post(
            RR_REMOVE,
            &[
                ("guild", "7"),
                ("channel_id", "20"),
                ("message_id", "1"),
                ("emoji", "x"),
                ("extra", "1"),
            ],
            Some(ADMIN),
        )
        .await
        .unwrap();
    assert_eq!(unknown.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(unknown.html.contains("unknown field `extra`"), "{}", unknown.html);
    let aimed = app
        .post(
            RR_REMOVE,
            &[
                ("guild", "11"),
                ("channel_id", "99"),
                ("message_id", "1"),
                ("emoji", "\u{1f3ae}"),
            ],
            Some(ADMIN),
        )
        .await
        .unwrap();
    assert_eq!(aimed.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        aimed.html.contains("the form's guild is not the page's guild"),
        "{}",
        aimed.html
    );
    assert_eq!(app.mappings(7).await.unwrap().len(), 2);
    assert_eq!(writes(&app), sent);

    for path in [
        "/guild/7/reaction-roles",
        "/guild/7/reaction-roles?action=",
        "/guild/7/reaction-roles?action=purge",
    ] {
        let reply = app.post(path, &[("guild", "7")], Some(ADMIN)).await.unwrap();
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
    }

    let signed_out = app.post(RR_ADD, &[("guild", "7")], None).await.unwrap();
    assert!(login_redirect(&signed_out));
    assert!(login_redirect(
        &app.get("/guild/7/reaction-roles", None).await.unwrap()
    ));

    let member = app.get("/guild/7/reaction-roles", Some(MEMBER)).await.unwrap();
    assert_eq!(member.status, StatusCode::OK);
    assert!(
        member.html.contains(
            r#"<p class="error">Failed to load reaction roles: error running server function: forbidden</p>"#
        ),
        "{}",
        member.html
    );
    assert!(!member.html.contains("Add a mapping"), "{}", member.html);
    let refused = app
        .post(
            RR_REMOVE,
            &[
                ("guild", "7"),
                ("channel_id", "99"),
                ("message_id", "1"),
                ("emoji", "\u{1f3ae}"),
            ],
            Some(MEMBER),
        )
        .await
        .unwrap();
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        refused.html.contains("Failed to load reaction roles"),
        "{}",
        refused.html
    );
    assert!(!refused.html.contains("alert error"), "{}", refused.html);
    assert_eq!(app.mappings(7).await.unwrap().len(), 2);

    app.pool.close().await;
}

const GR: &str = "/guild/7/greetings";

fn action(name: &str) -> String {
    format!("{GR}?action={name}")
}

const MESSAGES_FORM_HEAD: &str = "Messages</legend>";
const LEGEND_LIST: &str = r#"<ul class="greet-legend"><li><code>{user}</code> - mentions the person being greeted, or the sender when the command is run without a user.</li><li><code>{author}</code> - mentions whoever ran the command.</li><li>Leave a message blank to post just the image.</li></ul>"#;
const SAVE_BUTTON: &str = "<div class=\"form-actions\"><button type=\"submit\" class=\"btn btn-primary\" data-pending-label=\"Saving\u{2026}\">Save</button></div></form>";
const NO_IMAGES: &str = r#"<div class="empty">No images yet - the command will reply with just the message until you add one.</div>"#;

fn message_input(label: &str, name: &str, value: &str) -> String {
    format!(
        r#"<div class="setting-field"><label>{label}</label><input class="input" type="text" name="{name}" value="{value}" placeholder="(not set)" pattern=".*"></div>"#
    )
}

fn cooldown_input(label: &str, name: &str, value: &str) -> String {
    format!(
        r#"<div class="setting-field"><label>{label}</label><input class="input" type="text" name="{name}" value="{value}" placeholder="(not set)" pattern="[0-9]*"></div>"#
    )
}

fn messages_form(morning: &str, night: &str) -> String {
    format!(
        r#"<form method="post" action="{}" data-pending=""><input type="hidden" name="guild" value="7">{}{}{LEGEND_LIST}{SAVE_BUTTON}"#,
        action("save-messages"),
        message_input("Good morning message", "morning_message", morning),
        message_input("Good night message", "night_message", night),
    )
}

fn cooldowns_form(user: &str, guild: &str, floors: (u32, u32)) -> String {
    format!(
        r#"<form method="post" action="{}" data-pending=""><input type="hidden" name="guild" value="7">{}{}{SAVE_BUTTON}"#,
        action("save-cooldowns"),
        cooldown_input(
            &format!("Per-member cooldown (seconds, min {})", floors.0),
            "user_cooldown",
            user
        ),
        cooldown_input(
            &format!("Server-wide cooldown (seconds, min {})", floors.1),
            "guild_cooldown",
            guild
        ),
    )
}

fn image_form(kind: &str) -> String {
    format!(
        r#"<form method="post" action="{}" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="kind" value="{kind}"><div class="setting-field"><label>Image link</label><input class="input" type="url" name="url" placeholder="https://example.com/sunrise.gif" pattern="https://.*" required=""></div><div class="form-actions"><button type="submit" class="btn btn-primary">"#,
        action("add-image")
    )
}

const CHANNEL_SELECT_OPTIONS: &str = "<option value=\"\" selected=\"\">(not set)</option><option value=\"23\">\u{25b8} Lounge</option><option value=\"20\"># rules</option><option value=\"21\"># chat</option><option value=\"22\"># bots</option>";
const SEE_PLANS: &str = r#"<p class="page-lead">On Pro these floors drop to 3s and 1s. <a href="/upgrade">See plans</a>.</p>"#;
const GOOD_WORKS_LEAD: &str = "Where /good works</legend><p class=\"page-lead\">With nothing listed, <code>/good</code> works in every channel. Add one or more and Discord hides the command everywhere else - it never even shows up in the picker. Adding a category covers every channel inside it.</p><p class=\"page-lead\">This writes the same command permissions as Discord's own Server Settings \u{2192} Integrations panel, so changes made either way show up in both.</p>";

#[sqlx::test(migrations = "../migrations")]
async fn the_greetings_page_renders_every_section_for_a_member_admin(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();
    app.mock_free_guild(7);
    app.guild_directory(7);

    let html = app.page(GR).await.unwrap();
    assert!(html.contains(&title(GREETINGS_TITLE)), "{html}");
    assert!(html.contains("<h1>Greetings</h1>"), "{html}");
    assert!(
        html.contains(
            r#"<p class="page-lead">What Zayden posts for <code>/good morning</code> and <code>/good night</code>. Each subcommand replies with one image picked at random from its list, plus the message below if you set one.</p>"#
        ),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            "{MESSAGES_FORM_HEAD}{}</fieldset>",
            messages_form("", "")
        )),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            r#"{GOOD_WORKS_LEAD}<div class="chip-list"></div><form class="chip-add" method="post" action="{}" data-pending=""><input type="hidden" name="guild" value="7"><div class="setting-field"><label>Allow a channel</label><div class="select"><select class="input" name="channel_id">{CHANNEL_SELECT_OPTIONS}</select>"#,
            action("add-channel")
        )),
        "{html}"
    );
    assert!(
        html.contains(
            r#"<button type="submit" class="btn btn-ghost">Add channel</button></form></fieldset>"#
        ),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            r#"Cooldowns</legend><p class="page-lead">The per-member cooldown stops one person spamming <code>/good</code>; the server-wide one stops a crowd doing it between them. Both are in seconds, and both must stay at or above the minimum for this server's plan.</p>{}{SEE_PLANS}</fieldset>"#,
            cooldowns_form("15", "3", (15, 3))
        )),
        "{html}"
    );
    for (heading, kind) in
        [("Good morning images", "morning"), ("Good night images", "night")]
    {
        assert!(
            html.contains(&format!(
                "{heading}</legend>{NO_IMAGES}{}",
                image_form(kind)
            )),
            "{html}"
        );
    }
    assert_eq!(count(&html, "Up to 50 per greeting.</p></fieldset>"), 2, "{html}");
    assert_eq!(count(&html, "alert"), 0, "{html}");
    assert!(!html.contains("module-locked"), "{html}");

    assert_eq!(app.app.entitlements.guild_tier(7).await, Tier::Free);

    let added = app.page("/guild/7/greetings?channel_added=1").await.unwrap();
    let alert_at = added.find(SAVED).expect("saved alert");
    assert_eq!(count(&added, SAVED), 1, "{added}");
    assert!(added.find(r#"<div class="chip-list">"#).unwrap() < alert_at, "{added}");
    assert!(alert_at < added.find(r#"<form class="chip-add""#).unwrap(), "{added}");
    let removed = app.page("/guild/7/greetings?channel_removed=1").await.unwrap();
    assert_eq!(count(&removed, SAVED), 1, "{removed}");
    for query in ["channel_added=0", "image_added=", "saved=1"] {
        let page = app.page(&format!("/guild/7/greetings?{query}")).await.unwrap();
        assert_eq!(count(&page, SAVED), 0, "{query}");
    }

    app.mock_free_guild(12);
    app.guild_directory(12);
    app.app
        .entitlements
        .grant(EntitlementScope::Guild(12), Tier::Pro, "test", "pro-12", None)
        .await
        .unwrap();
    assert_eq!(app.app.entitlements.guild_tier(12).await, Tier::Pro);
    let pro = app.page("/guild/12/greetings").await.unwrap();
    assert!(pro.contains("Per-member cooldown (seconds, min 3)"), "{pro}");
    assert!(pro.contains("Server-wide cooldown (seconds, min 1)"), "{pro}");
    assert!(!pro.contains("See plans"), "{pro}");
    assert!(!pro.contains("these floors drop"), "{pro}");
    let below = app
        .post(
            "/guild/12/greetings?action=save-cooldowns",
            &[("guild", "12"), ("user_cooldown", "2"), ("guild_cooldown", "1")],
            Some(ADMIN),
        )
        .await
        .unwrap();
    assert_eq!(below.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        below.html.contains(&format!(
            "{FAILED_SAVE}On the Pro plan the per-member cooldown can't go below 3s. That is as low as this command goes.</span>"
        )),
        "{}",
        below.html
    );

    app.discord.reply(
        "GET",
        &format!("/applications/{APP_ID}/guilds/7/commands"),
        500,
        "{}",
    );
    let unknown = app.page(GR).await.unwrap();
    assert!(
        unknown.contains(
            r#"<div class="chip-list"></div><p class="module-locked">Discord didn't report which channels <code>/good</code> is allowed in, so its restrictions can't be shown or changed right now.</p></fieldset>"#
        ),
        "{unknown}"
    );
    assert!(!unknown.contains("With nothing listed"), "{unknown}");
    assert!(!unknown.contains("chip-add"), "{unknown}");
    assert!(
        unknown.contains("This writes the same command permissions"),
        "{unknown}"
    );

    let member = app.get(GR, Some(MEMBER)).await.unwrap();
    assert_eq!(member.status, StatusCode::OK);
    assert!(
        member.html.contains(
            r#"<p class="error">Failed to load greetings: error running server function: forbidden</p>"#
        ),
        "{}",
        member.html
    );
    assert!(!member.html.contains("Messages</legend>"), "{}", member.html);
    assert!(login_redirect(&app.get(GR, None).await.unwrap()));
    let malformed = app.get("/guild/abc/greetings", Some(ADMIN)).await.unwrap();
    assert!(
        malformed.html.contains(
            "Failed to load greetings: error running server function: invalid guild id"
        ),
        "{}",
        malformed.html
    );

    app.pool.close().await;
}

async fn submit(
    app: &Harness,
    name: &str,
    fields: Vec<(&str, &str)>,
) -> TestResult<Answer> {
    let mut all = vec![("guild", "7")];
    all.extend(fields);
    app.post(&action(name), &all, Some(ADMIN)).await
}

fn image_card(url: &str, id: &str) -> String {
    format!(
        r#"<div class="greet-card"><img class="greet-thumb" src="{url}" alt="" loading="lazy"><a class="greet-url" href="{url}" rel="external noreferrer" target="_blank" title="{url}">{url}</a><form class="greet-remove" method="post" action="{}" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="id" value="{id}"><button type="submit" class="btn btn-ghost">"#,
        action("remove-image")
    )
}

fn saved_before(html: &str, later: &str) -> bool {
    match (html.find(SAVED), html.find(later)) {
        (Some(alert), Some(at)) => alert < at,
        _ => false,
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn the_greetings_saves_rerender_the_page_with_their_result(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();
    app.mock_free_guild(7);
    app.guild_directory(7);

    let failure = |html: &str, message: &str| {
        html.contains(&format!("{FAILED_SAVE}{message}</span>"))
    };

    let saved = submit(&app, "save-messages", vec![
        ("morning_message", "  Good morning {user}!  "),
        ("night_message", "Night, {author}"),
    ])
    .await
    .unwrap();
    assert_eq!(saved.status, StatusCode::OK);
    assert!(saved.html.contains(&title(GREETINGS_TITLE)), "{}", saved.html);
    assert!(
        saved.html.contains(&format!(
            "{MESSAGES_FORM_HEAD}{SAVED_ALERT}{}",
            messages_form("Good morning {user}!", "Night, {author}")
        )),
        "{}",
        saved.html
    );
    assert_eq!(count(&saved.html, SAVED), 1, "{}", saved.html);
    assert_eq!(count(&saved.html, "alert error"), 0, "{}", saved.html);

    let too_long = "x".repeat(1501);
    let refused = submit(&app, "save-messages", vec![
        ("morning_message", &too_long),
        ("night_message", "kept?"),
    ])
    .await
    .unwrap();
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        failure(
            &refused.html,
            "Greeting messages are limited to 1500 characters so the reply still fits once mentions are filled in."
        ),
        "{}",
        refused.html
    );
    assert!(
        refused
            .html
            .contains(&messages_form("Good morning {user}!", "Night, {author}")),
        "{}",
        refused.html
    );

    let cooldowns = submit(&app, "save-cooldowns", vec![
        ("user_cooldown", "30"),
        ("guild_cooldown", " 10 "),
    ])
    .await
    .unwrap();
    assert_eq!(cooldowns.status, StatusCode::OK);
    assert!(saved_before(&cooldowns.html, &cooldowns_form("30", "10", (15, 3))));
    assert!(cooldowns.html.contains(SEE_PLANS), "{}", cooldowns.html);

    let below = submit(&app, "save-cooldowns", vec![
        ("user_cooldown", "5"),
        ("guild_cooldown", ""),
    ])
    .await
    .unwrap();
    assert_eq!(below.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        failure(
            &below.html,
            "On the Free plan the per-member cooldown can't go below 15s. Pro servers can go as low as 3s."
        ),
        "{}",
        below.html
    );
    assert!(
        below.html.contains(&cooldowns_form("30", "10", (15, 3))),
        "{}",
        below.html
    );
    let junk = submit(&app, "save-cooldowns", vec![
        ("user_cooldown", "abc"),
        ("guild_cooldown", "3"),
    ])
    .await
    .unwrap();
    assert!(
        failure(
            &junk.html,
            "`abc` isn't a usable cooldown. Enter a whole number of seconds between 0 and 86400."
        ),
        "{}",
        junk.html
    );

    let first = "https://example.com/a.gif";
    let created =
        submit(&app, "add-image", vec![("kind", "morning"), ("url", first)])
            .await
            .unwrap();
    assert_eq!(created.status, StatusCode::SEE_OTHER);
    assert_eq!(
        created.location.as_deref(),
        Some("/guild/7/greetings?image_added=1")
    );
    let added =
        app.get("/guild/7/greetings?image_added=1", Some(ADMIN)).await.unwrap();
    assert_eq!(added.status, StatusCode::OK);
    assert_eq!(app.images(7, "morning").await.unwrap(), [first]);
    assert!(
        saved_before(&added.html, "Good morning images</legend>"),
        "{}",
        added.html
    );
    assert_eq!(count(&added.html, SAVED), 1, "{}", added.html);
    let id =
        attribute_after(&added.html, r#"<input type="hidden" name="id" value=""#)
            .expect("image id")
            .to_owned();
    assert!(
        added
            .html
            .contains(&format!("{}<svg class=\"icon\"", image_card(first, &id))),
        "{}",
        added.html
    );
    assert!(
        added.html.contains(&format!("{}{NO_IMAGES}", "Good night images</legend>"))
    );
    assert_eq!(count(&added.html, r#"class="greet-grid""#), 1, "{}", added.html);

    let duplicate =
        submit(&app, "add-image", vec![("kind", "morning"), ("url", first)])
            .await
            .unwrap();
    assert_eq!(duplicate.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(failure(&duplicate.html, "That image link is already in the list."));
    let insecure = submit(&app, "add-image", vec![
        ("kind", "night"),
        ("url", "http://example.com/a.gif"),
    ])
    .await
    .unwrap();
    assert!(
        failure(
            &insecure.html,
            "`http://example.com/a.gif` isn't a usable image link. Links must start with `https://`."
        ),
        "{}",
        insecure.html
    );
    let unknown_kind =
        submit(&app, "add-image", vec![("kind", "noon"), ("url", first)])
            .await
            .unwrap();
    assert!(failure(&unknown_kind.html, "Unknown greeting type `noon`."));
    assert_eq!(app.images(7, "night").await.unwrap().len(), 0);

    let missing =
        submit(&app, "remove-image", vec![("id", "999999")]).await.unwrap();
    assert_eq!(missing.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(failure(&missing.html, "that image is not in this server's list"));
    let done = submit(&app, "remove-image", vec![("id", &id)]).await.unwrap();
    assert_eq!(done.status, StatusCode::SEE_OTHER);
    assert_eq!(done.location.as_deref(), Some("/guild/7/greetings?image_removed=1"));
    let removed =
        app.get("/guild/7/greetings?image_removed=1", Some(ADMIN)).await.unwrap();
    assert_eq!(removed.status, StatusCode::OK);
    assert_eq!(app.images(7, "morning").await.unwrap().len(), 0);
    assert!(saved_before(&removed.html, "Good morning images</legend>"));
    assert!(
        removed.html.contains(&format!("Good morning images</legend>{NO_IMAGES}"))
    );

    let unregistered =
        submit(&app, "add-channel", vec![("channel_id", "20")]).await.unwrap();
    assert_eq!(unregistered.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        unregistered.html.contains(&format!(
            "<div class=\"chip-list\"></div>{FAILED_SAVE}/good isn't registered for this server yet</span>"
        )),
        "{}",
        unregistered.html
    );
    let foreign =
        submit(&app, "add-channel", vec![("channel_id", "99")]).await.unwrap();
    assert!(failure(&foreign.html, "that channel is not in this server"));
    let removed =
        submit(&app, "remove-channel", vec![("channel_id", "20")]).await.unwrap();
    assert!(failure(&removed.html, "/good isn't registered for this server yet"));

    let writes = |app: &Harness| {
        ["POST", "PUT", "DELETE"].map(|method| app.discord.count(method, "/"))
    };
    let sent = writes(&app);
    let missing = app
        .post(
            &action("save-messages"),
            &[("guild", "7"), ("morning_message", "x")],
            Some(ADMIN),
        )
        .await
        .unwrap();
    assert_eq!(missing.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        failure(&missing.html, "missing field `night_message`"),
        "{}",
        missing.html
    );
    let aimed = app
        .post(&action("remove-image"), &[("guild", "11"), ("id", &id)], Some(ADMIN))
        .await
        .unwrap();
    assert_eq!(aimed.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(writes(&app), sent);

    for path in [GR, "/guild/7/greetings?action=", "/guild/7/greetings?action=reset"]
    {
        let reply = app.post(path, &[("guild", "7")], Some(ADMIN)).await.unwrap();
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
    }
    let signed_out =
        app.post(&action("save-messages"), &[("guild", "7")], None).await.unwrap();
    assert!(login_redirect(&signed_out));
    let member = app
        .post(
            &action("add-image"),
            &[("guild", "7"), ("kind", "morning"), ("url", first)],
            Some(MEMBER),
        )
        .await
        .unwrap();
    assert_eq!(member.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(member.html.contains(
        "Failed to load greetings: error running server function: forbidden"
    ));
    assert_eq!(app.images(7, "morning").await.unwrap().len(), 0);

    app.pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn operators_see_the_channel_list_read_only(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();
    app.discord.get("/guilds/8", &guild_json(8, 1));
    app.discord.get(
        &format!("/applications/{APP_ID}/commands"),
        &list(&[command_json(400, "ping"), command_json(500, "good")]),
    );
    app.discord.get(&format!("/applications/{APP_ID}/guilds/8/commands"), "[]");
    app.discord.get(
        &format!("/applications/{APP_ID}/guilds/8/commands/500/permissions"),
        &permissions_json(8, 500, &[
            (7, 3, false),
            (21, 3, true),
            (22, 3, true),
            (31, 1, false),
        ]),
    );
    app.guild_directory(8);

    let html = app.get("/guild/8/greetings", Some(OPERATOR)).await.unwrap().html;
    assert!(
        html.contains(
            r#"<div class="chip-list"><span class="chip"><span class="chip-label">#chat</span></span><span class="chip"><span class="chip-label">#bots</span></span></div><p class="module-locked">Read-only: Discord only lets a member with Manage Server change which channels a command is allowed in.</p></fieldset>"#
        ),
        "{html}"
    );
    assert!(html.contains("With nothing listed"), "{html}");
    assert!(!html.contains("chip-add"), "{html}");
    assert!(!html.contains("chip-remove"), "{html}");
    assert!(
        html.contains(
            r#"<form method="post" action="/guild/8/greetings?action=save-messages""#
        ),
        "{html}"
    );

    let refused = app
        .post(
            "/guild/8/greetings?action=add-channel",
            &[("guild", "8"), ("channel_id", "20")],
            Some(OPERATOR),
        )
        .await
        .unwrap();
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(!refused.html.contains("alert error"), "{}", refused.html);
    assert!(refused.html.contains("module-locked"), "{}", refused.html);
    assert_eq!(app.discord.count("PUT", "/"), 0);

    let admin = app.get("/guild/8/greetings", Some(ADMIN)).await.unwrap();
    assert!(
        admin.html.contains(
            "Failed to load greetings: error running server function: forbidden"
        ),
        "{}",
        admin.html
    );

    app.pool.close().await;
}

path_param!(variant);

#[page("/test/channels/{variant}")]
async fn channel_variants(cx: &Cx) -> ViewResult<impl View> {
    let variant: &str = path_param::<Variant>(cx);
    let channels = [
        ChannelInfo {
            id: "21".to_owned(),
            name: "chat".to_owned(),
            kind: ChannelType::GuildText,
            tags: Vec::new(),
        },
        ChannelInfo {
            id: "22".to_owned(),
            name: "bots".to_owned(),
            kind: ChannelType::GuildText,
            tags: Vec::new(),
        },
    ];
    let allowed = ["21".to_owned(), "99".to_owned()];
    let feedback = (variant == "editable").then_some(Err("boom"));

    Ok(view! {
        channel_section(
            guild_id: "7",
            allowed: (variant != "unknown").then_some(allowed.as_slice()),
            channels: &channels,
            locked: variant == "locked",
            added: feedback,
            removed: (variant == "editable").then_some(Ok(()))
        )
    })
}

async fn standalone(path: &str) -> TestResult<String> {
    let base = Router::builder()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .page(channel_variants);
    let router = web::router(base);
    let response = router.handle(Request::get(path).body(Body::empty())?).await;
    body_text(response).await
}

#[tokio::test]
async fn the_channel_section_lists_removable_chips_for_a_member() {
    let html = standalone("/test/channels/editable").await.unwrap();
    let remove = r#"<form class="chip" method="post" action="/guild/7/greetings?action=remove-channel" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="channel_id" value="21"><span class="chip-label">#chat</span><button type="submit" class="chip-remove" title="Remove"><svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg></button></form>"#;
    assert!(
        html.contains(&format!(
            r#"<div class="chip-list">{remove}{}"#,
            remove
                .replace("channel_id\" value=\"21", "channel_id\" value=\"99")
                .replace("#chat", "#unknown (99)")
        )),
        "{html}"
    );
    assert!(
        html.contains(
            "</div><div class=\"alert success\" role=\"status\"><span>Saved.</span>"
        ),
        "{html}"
    );
    let removed_at = html.find(SAVED).unwrap();
    let added_at = html.find(&format!("{FAILED_SAVE}boom</span>")).unwrap();
    assert!(removed_at < added_at, "{html}");
    assert!(added_at < html.find("chip-add").unwrap(), "{html}");
    assert!(
        html.contains(r#"<option value="22"># bots</option></select>"#)
            && !html.contains(r#"<option value="21">"#),
        "{html}"
    );

    let locked = standalone("/test/channels/locked").await.unwrap();
    assert!(
        locked.contains(
            r#"<span class="chip"><span class="chip-label">#chat</span></span><span class="chip"><span class="chip-label">#unknown (99)</span></span></div><p class="module-locked">Read-only"#
        ),
        "{locked}"
    );
    assert!(!locked.contains("chip-remove"), "{locked}");
    assert!(!locked.contains("alert"), "{locked}");

    let unknown = standalone("/test/channels/unknown").await.unwrap();
    assert!(
        unknown.contains(
            r#"<div class="chip-list"></div><p class="module-locked">Discord didn't report which channels"#
        ),
        "{unknown}"
    );
    assert!(!unknown.contains("With nothing listed"), "{unknown}");
}

#[tokio::test]
async fn an_unreachable_database_leaves_each_page_with_its_load_error() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy("postgres://127.0.0.1:1/unused")
        .unwrap();
    let app = harness_without_database(pool).unwrap();

    for (path, heading) in [
        ("/guild/7/levels", "Failed to load leaderboard: "),
        ("/guild/7/reaction-roles", "Failed to load reaction roles: "),
        ("/guild/7/greetings", "Failed to load greetings: "),
    ] {
        let reply = app.get(path, Some("session=uncached-token")).await.unwrap();
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        assert!(
            reply.html.contains(&format!(
                r#"<p class="error">{heading}error running server function: "#
            )),
            "{path}: {}",
            reply.html
        );
        assert!(!reply.html.contains("Add a mapping"), "{path}");
    }
}
