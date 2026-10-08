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
use web::auth::{ChannelInfo, SessionUser};
use web::components::brand::LOGO;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::engagement::pages::greetings::channel_section;
use web::engagement::pages::{GREETINGS_TITLE, LEVELS_TITLE, REACTION_ROLES_TITLE};
use web::state::{DiscordState, SessionIdentity, SessionUsersCache, WebState};
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
    flash: Option<String>,
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
        users: state.discord.users,
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
    seed_users(&state.discord.users, &[41, 43, 44]).await;
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
        let flash = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|value| {
                value.starts_with("flash=") && !value.starts_with("flash=;")
            })
            .and_then(|value| value.split(';').next())
            .map(str::to_owned);
        Ok(Answer { status, html: body_text(response).await?, location, flash })
    }

    /// Opens the page a 303 leads to, carrying the flash it set.
    async fn follow(&self, reply: &Answer) -> TestResult<Answer> {
        let location = reply.location.as_deref().ok_or("no Location")?;
        let page = location.split('#').next().unwrap_or(location);
        let flash = reply.flash.as_deref().ok_or("the redirect set no flash")?;
        self.get(page, Some(&format!("{ADMIN}; {flash}"))).await
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
        r#"<tr role="row"><td role="cell" data-label="Rank" class="num">{rank}</td><th scope="row" role="rowheader" data-label="Member">{avatar}<span class="lb-name">{name}</span></th><td role="cell" data-label="Level" class="num">{level}</td><td role="cell" data-label="XP" class="num">{xp}</td><td role="cell" data-label="Messages" class="num">{messages}</td></tr>"#
    )
}

const PLACEHOLDER: &str = r#"<span class="lb-avatar placeholder"></span>"#;
const ENTRY: &str = r#"role="rowheader" data-label="Member""#;

fn board_head(caption: &str) -> String {
    format!(
        r#"<div class="data-table-wrap"><table class="data-table" role="table"><caption class="visually-hidden">{caption}</caption><thead role="rowgroup"><tr role="row"><th scope="col" role="columnheader">Rank</th><th scope="col" role="columnheader">Member</th><th scope="col" role="columnheader">Level</th><th scope="col" role="columnheader">XP</th><th scope="col" role="columnheader">Messages</th></tr></thead><tbody role="rowgroup">"#
    )
}

fn avatar(id: u64, hash: &str) -> String {
    format!(
        r#"<img class="lb-avatar" src="https://cdn.discordapp.com/avatars/{id}/{hash}.png" alt="">"#
    )
}

fn scope(guild: u64, global: bool) -> String {
    let (server, world) = if global {
        (r#"class="seg""#, r#"class="seg active""#)
    } else {
        (r#"class="seg active""#, r#"class="seg""#)
    };
    let (server_current, world_current) = if global {
        ("", r#" aria-current="page""#)
    } else {
        (r#" aria-current="page""#, "")
    };
    format!(
        r#"<nav class="segmented" aria-label="Leaderboard"><a {server} href="/guild/{guild}/levels"{server_current}>This server</a><a {world} href="/guild/{guild}/levels?scope=global"{world_current}>Global</a></nav>"#
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
        r#"<nav class="pager" aria-label="Leaderboard pages">{previous}<span class="pager-page">Page {page}</span>{next}</nav>"#
    )
}

fn empty_board(title: &str, text: &str) -> String {
    format!(
        r#"<div class="empty-state"><h2 class="empty-title">{title}</h2><p class="empty-text">{text}</p></div>"#
    )
}

fn past_the_end(first: &str) -> String {
    format!(
        r#"<div class="empty-state"><h2 class="empty-title">No one on this page</h2><p class="empty-text">No more entries on this page.</p><a href="{first}" class="btn btn-primary">Back to page 1</a></div>"#
    )
}

/// The in-page error panel of a page that could not load its data.
fn load_error(title: &str, message: &str) -> String {
    format!(
        r#"<section class="error-panel" role="alert" aria-labelledby="load-error-title"><h2 class="error-title" id="load-error-title">{title}</h2><p class="error-text">{message}</p><div class="error-actions">"#
    )
}

const FORBIDDEN: &str = "You need Manage Server in this server to open this page.";
const NOT_A_SERVER: &str = "That address doesn't name a Discord server.";

#[sqlx::test(migrations = "../migrations")]
async fn the_levels_page_pages_the_board_by_links_and_names_members_from_discord(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();

    let empty = app.page("/guild/11/levels").await.unwrap();
    assert!(empty.contains(&title(LEVELS_TITLE)), "{empty}");
    assert!(
        empty.contains(
            r#"<div class="page-header"><div><h1>Levels</h1><p class="page-lead">Message-XP rankings. Switch between this server and the global board.</p></div></div><div class="flash-region" role="status"></div>"#
        ),
        "{empty}"
    );
    assert!(empty.contains(&scope(11, false)), "{empty}");
    assert!(
        empty.contains(&empty_board(
            "No one on the board yet",
            "No one has chatted here yet - the board fills as members talk."
        )),
        "{empty}"
    );
    assert!(!empty.contains("data-table"), "{empty}");
    assert!(!empty.contains(r#"class="pager""#), "{empty}");
    let empty = app.page("/guild/11/levels?scope=global").await.unwrap();
    assert!(empty.contains(&scope(11, true)), "{empty}");
    assert!(
        empty.contains(&empty_board(
            "No one on the board yet",
            "No one has earned global XP yet."
        )),
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
        first.contains(&board_head("This server leaderboard, page 1")),
        "{first}"
    );
    assert_eq!(count(&first, ENTRY), 10, "{first}");
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
            r#"<a class="seg active" href="/guild/7%3Fscope%3Dglobal/levels" aria-current="page">"#
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
    assert_eq!(count(&second, ENTRY), 2, "{second}");
    assert!(
        second.contains(&board_head("This server leaderboard, page 2")),
        "{second}"
    );
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
    assert!(past.contains(&past_the_end("/guild/7/levels")), "{past}");
    assert!(!past.contains("data-table"), "{past}");
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
    assert_eq!(count(&world, ENTRY), 2, "{world}");
    assert!(world.contains(&board_head("Global leaderboard, page 1")), "{world}");
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
        paged.contains(&past_the_end("/guild/7/levels?scope=global")),
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
        member
            .html
            .contains(&load_error("Couldn't load the leaderboard", FORBIDDEN)),
        "{}",
        member.html
    );
    assert!(!member.html.contains("data-table"), "{}", member.html);
    assert!(
        !member.html.contains("error running server function"),
        "{}",
        member.html
    );

    let outsider = app.get("/guild/9/levels", Some(ADMIN)).await.unwrap();
    assert!(
        outsider
            .html
            .contains(&load_error("Couldn't load the leaderboard", FORBIDDEN))
    );
    let malformed = app.get("/guild/abc/levels", Some(ADMIN)).await.unwrap();
    assert!(
        malformed
            .html
            .contains(&load_error("Couldn't load the leaderboard", NOT_A_SERVER)),
        "{}",
        malformed.html
    );

    app.pool.close().await;
}

const RR_LEAD: &str = r#"<p class="page-lead">Every message → emoji → role mapping in this server, in one place."#;
const EXTERNAL: &str = r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg>"#;
const CHECK: &str = r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>"#;
const EMPTY_FLASH: &str = r#"<div class="flash-region" role="status"></div>"#;

fn rr_row(
    index: usize,
    channel: &str,
    emoji: &str,
    role: &str,
    link: &str,
    shown_emoji: &str,
    fields: [&str; 3],
) -> String {
    let [channel_id, message_id, emoji_value] = fields;
    format!(
        r#"<tr role="row"><th scope="row" role="rowheader" data-label="Channel">{channel}</th><td role="cell" data-label="Emoji">{emoji}</td><td role="cell" data-label="Role">{role}</td><td role="cell" data-label="Message"><a class="rr-link" href="{link}" rel="external noreferrer" target="_blank">Open in Discord{EXTERNAL}</a></td><td role="cell" data-label="Action"><form method="post" action="{RR_REMOVE}" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="channel_id" value="{channel_id}"><input type="hidden" name="message_id" value="{message_id}"><input type="hidden" name="emoji" value="{emoji_value}"><div class="confirm" data-confirm=""><button type="submit" class="btn btn-ghost" data-confirm-trigger="">Remove</button><dialog class="dialog" aria-labelledby="rr-{index}-remove-title" aria-describedby="rr-{index}-remove-desc" data-confirm-dialog=""><div class="dialog-panel"><h2 class="dialog-title" id="rr-{index}-remove-title">Remove the {shown_emoji} reaction role for {role}?</h2><p class="dialog-desc" id="rr-{index}-remove-desc">Reactions already on the message stay, but they stop granting the role.</p><div class="dialog-actions"><button type="button" class="btn btn-secondary" data-dialog-close="" autofocus="">Cancel</button><button type="submit" class="btn btn-danger">Remove reaction role</button></div></div></dialog></div></form></td></tr>"#
    )
}

/// The result line a section shows after a successful post.
fn flashed(message: &str) -> String {
    format!(
        r#"<div class="flash-region" role="status"><p class="flash flash-success" data-flash="">{CHECK}<span class="flash-text">{message}</span></p></div>"#
    )
}

/// The reason at the top of form `form`.
fn summary(form: &str, outcome: &str, message: &str) -> String {
    format!(
        r#"<div class="error" id="{form}-summary" role="alert" tabindex="-1" autofocus="">{outcome}: {message}</div>"#
    )
}

const RR_ADD: &str = "/guild/7/reaction-roles/add";
const RR_REMOVE: &str = "/guild/7/reaction-roles/remove";
const RR_TABLE: &str = r#"<div class="data-table-wrap"><table class="data-table" role="table"><caption class="visually-hidden">Reaction roles</caption><thead role="rowgroup"><tr role="row"><th scope="col" role="columnheader">Channel</th><th scope="col" role="columnheader">Emoji</th><th scope="col" role="columnheader">Role</th><th scope="col" role="columnheader">Message</th><th scope="col" role="columnheader">Action</th></tr></thead><tbody role="rowgroup">"#;
const RR_ENTRY: &str = r#"role="rowheader" data-label="Channel""#;

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
    assert!(empty.contains("<h1>Reaction roles</h1>"), "{empty}");
    assert!(empty.contains(RR_LEAD), "{empty}");
    assert!(
        empty.contains(&format!(
            r#"<section class="settings-section" id="add-reaction-role" aria-labelledby="add-reaction-role-title"><h2 class="label" id="add-reaction-role-title">Add a reaction role</h2><p class="page-lead">Leave the message ID blank and Zayden posts a new panel message in the chosen channel. Give an ID to attach the mapping to a message that already exists - several emoji can share one message.</p>{EMPTY_FLASH}<form method="post" action="{RR_ADD}" data-pending="" data-dirty-guard=""><input type="hidden" name="guild" value="7">"#
        )),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<label class="field-label" for="add-reaction-role-channel-id">Channel</label><div class="select"><select class="input" id="add-reaction-role-channel-id" name="channel_id" required="" aria-describedby="add-reaction-role-channel-id-help"><option value="" selected="">(not set)</option><option value="20"># rules</option><option value="21"># chat</option><option value="22"># bots</option></select>"#
        ),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<label class="field-label" for="add-reaction-role-message-id">Message ID</label><input class="input" id="add-reaction-role-message-id" type="text" name="message_id" value="" inputmode="numeric" pattern="[0-9]*" autocomplete="off" aria-describedby="add-reaction-role-message-id-help"><p class="field-help" id="add-reaction-role-message-id-help">Optional. Leave blank to post a new panel message.</p>"#
        ),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<label class="field-label" for="add-reaction-role-emoji">Emoji</label><input class="input" id="add-reaction-role-emoji" type="text" name="emoji" value="" placeholder="✅ or <:name:id>" required="" autocomplete="off" aria-describedby="add-reaction-role-emoji-help">"#
        ),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<select class="input" id="add-reaction-role-role-id" name="role_id" required="" aria-describedby="add-reaction-role-role-id-help"><option value="" selected="">(not set)</option><option value="31">@Mod</option><option value="30">@Member</option></select>"#
        ),
        "{empty}"
    );
    assert!(
        empty.contains(
            r#"<p class="field-help" id="add-reaction-role-role-id-help">Zayden's role must be above the roles it assigns. In Discord, drag it above them under Server Settings &gt; Roles.</p>"#
        ),
        "{empty}"
    );
    assert!(
        empty.contains(&format!(
            r#"<section class="settings-section" id="reaction-roles" aria-labelledby="reaction-roles-title"><h2 class="label" id="reaction-roles-title">Mappings</h2>{EMPTY_FLASH}<p class="page-lead">No reaction roles yet - add one above and Zayden will seed the reaction for members to click.</p></section>"#
        )),
        "{empty}"
    );
    assert!(
        empty.find("Add a reaction role").unwrap() < empty.find("Mappings").unwrap(),
        "{empty}"
    );
    assert!(!empty.contains("data-table"), "{empty}");
    assert!(!empty.contains(r#"role="alert""#), "{empty}");

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
    assert!(
        unseeded.html.contains(r#"<div class="error" id="add-reaction-role-summary" role="alert" tabindex="-1" autofocus="">Not added: "#),
        "{}",
        unseeded.html
    );
    assert!(
        unseeded.html.contains("reaction_roles_guild_id_fkey"),
        "{}",
        unseeded.html
    );
    assert!(unseeded.html.contains(&title(REACTION_ROLES_TITLE)));
    assert!(
        unseeded.html.contains(r#"<option value="20" selected="">"#),
        "{}",
        unseeded.html
    );
    assert!(
        unseeded.html.contains(r#"name="emoji" value="✅""#),
        "{}",
        unseeded.html
    );
    assert!(unseeded.html.contains("No reaction roles yet"));
    assert!(
        !unseeded.html.contains("error running server function"),
        "{}",
        unseeded.html
    );
    assert_eq!(app.mappings(7).await.unwrap().len(), 0);

    app.seed_guild(7).await.unwrap();
    let created = add("", "30", "\u{2705}").await;
    assert_eq!(created.status, StatusCode::SEE_OTHER);
    assert_eq!(
        created.location.as_deref(),
        Some("/guild/7/reaction-roles#add-reaction-role")
    );
    assert_eq!(app.mappings(7).await.unwrap(), ["20/555/30/\u{2705}"]);
    assert_eq!(app.discord.count("POST", "/channels/20/messages"), 2);
    let saved = app.follow(&created).await.unwrap();
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(app.discord.count("POST", "/channels/20/messages"), 2);
    assert!(
        saved.html.contains(&format!(
            r#"several emoji can share one message.</p>{}<form method="post" action="{RR_ADD}""#,
            flashed("Reaction role added.")
        )),
        "{}",
        saved.html
    );
    assert_eq!(count(&saved.html, "flash-text"), 1, "{}", saved.html);
    assert!(saved.html.contains(RR_TABLE), "{}", saved.html);
    assert!(
        saved.html.contains(&rr_row(
            0,
            "#rules",
            r#"<span class="rr-emoji">✅</span>"#,
            "@Member",
            "https://discord.com/channels/7/20/555",
            "\u{2705}",
            ["20", "555", "\u{2705}"],
        )),
        "{}",
        saved.html
    );
    let again = app.page("/guild/7/reaction-roles").await.unwrap();
    assert!(!again.contains("flash-text"), "{again}");

    let duplicate = add("555", "31", "\u{2705}").await;
    assert_eq!(duplicate.status, StatusCode::UNPROCESSABLE_ENTITY);
    let message = "that emoji is already mapped on that message";
    assert!(
        duplicate.html.contains(&summary("add-reaction-role", "Not added", message)),
        "{}",
        duplicate.html
    );
    assert!(
        duplicate.html.contains(
            r#"aria-describedby="add-reaction-role-emoji-help add-reaction-role-emoji-error" aria-invalid="true"><p class="field-help" id="add-reaction-role-emoji-help">"#
        ),
        "{}",
        duplicate.html
    );
    assert!(
        duplicate.html.contains(&format!(
            r#"<p class="field-error" id="add-reaction-role-emoji-error">{message}</p>"#
        )),
        "{}",
        duplicate.html
    );
    assert_eq!(count(&duplicate.html, RR_ENTRY), 1, "{}", duplicate.html);

    let created = add("555", "31", "<:a:5>").await;
    assert_eq!(created.status, StatusCode::SEE_OTHER);
    let custom = app.follow(&created).await.unwrap();
    assert!(
        custom.html.contains(
            r#"<td role="cell" data-label="Emoji"><img class="rr-emoji-img" src="https://cdn.discordapp.com/emojis/5.png?size=32" alt=":a:" title=":a:"></td><td role="cell" data-label="Role">@Mod</td>"#
        ),
        "{}",
        custom.html
    );
    assert!(
        custom.html.contains("Remove the :a: reaction role for @Mod?"),
        "{}",
        custom.html
    );
    assert_eq!(count(&custom.html, RR_ENTRY), 2, "{}", custom.html);

    app.seed_mapping(7, 99, 1, 98, "\u{1f3ae}").await.unwrap();
    let listed = app.page("/guild/7/reaction-roles").await.unwrap();
    assert_eq!(count(&listed, RR_ENTRY), 3, "{listed}");
    assert!(
        listed.contains(r#"data-label="Channel">#unknown (99)</th>"#),
        "{listed}"
    );
    assert!(
        listed.contains(r#"<td role="cell" data-label="Role">@unknown (98)</td>"#),
        "{listed}"
    );
    assert!(!listed.contains(r#"role="alert""#), "{listed}");

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
    assert_eq!(
        done.location.as_deref(),
        Some("/guild/7/reaction-roles#reaction-roles")
    );
    assert_eq!(app.mappings(7).await.unwrap().len(), 2);
    let removed = app.follow(&done).await.unwrap();
    assert_eq!(removed.status, StatusCode::OK);
    assert!(
        removed.html.contains(&format!(
            r#"<h2 class="label" id="reaction-roles-title">Mappings</h2>{}{RR_TABLE}"#,
            flashed("Reaction role removed.")
        )),
        "{}",
        removed.html
    );
    assert_eq!(app.discord.count("DELETE", "/channels/20/messages/555/"), 1);

    for query in ["added=1", "removed=1", "added=1&removed=1", "other=1"] {
        let reply = app
            .get(&format!("/guild/7/reaction-roles?{query}"), Some(ADMIN))
            .await
            .unwrap();
        assert_eq!(reply.status, StatusCode::OK, "{query}");
        assert!(!reply.html.contains("flash-text"), "{query}");
    }

    let invalid = remove("x", "555", "\u{2705}").await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        invalid.html.contains(&summary(
            "remove-reaction-role",
            "Not removed",
            "invalid channel"
        )),
        "{}",
        invalid.html
    );
    assert_eq!(count(&invalid.html, r#"role="alert""#), 1, "{}", invalid.html);
    assert!(invalid.html.contains(&title(REACTION_ROLES_TITLE)));

    let invalid = add("", "", "\u{2705}").await;
    assert!(
        invalid.html.contains(&summary(
            "add-reaction-role",
            "Not added",
            "invalid role"
        )),
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
        missing.html.contains(&summary(
            "add-reaction-role",
            "Not added",
            "missing field `role_id`"
        )),
        "{}",
        missing.html
    );
    assert!(
        missing.html.contains(r#"<p class="field-error" id="add-reaction-role-role-id-error">missing field `role_id`</p>"#),
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
    assert!(
        unknown.html.contains("Not removed: unknown field `extra`"),
        "{}",
        unknown.html
    );
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
        aimed.html.contains("Not removed: the form's guild is not the page's guild"),
        "{}",
        aimed.html
    );
    assert_eq!(app.mappings(7).await.unwrap().len(), 2);
    assert_eq!(writes(&app), sent);

    let unknown_action = app
        .post("/guild/7/reaction-roles/purge", &[("guild", "7")], Some(ADMIN))
        .await
        .unwrap();
    assert_eq!(unknown_action.status, StatusCode::NOT_FOUND);

    let signed_out = app.post(RR_ADD, &[("guild", "7")], None).await.unwrap();
    assert!(login_redirect(&signed_out));
    assert!(login_redirect(
        &app.get("/guild/7/reaction-roles", None).await.unwrap()
    ));

    let member = app.get("/guild/7/reaction-roles", Some(MEMBER)).await.unwrap();
    assert_eq!(member.status, StatusCode::OK);
    assert!(
        member
            .html
            .contains(&load_error("Couldn't load the reaction roles", FORBIDDEN)),
        "{}",
        member.html
    );
    assert!(!member.html.contains("Add a reaction role"), "{}", member.html);
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
        refused.html.contains(&summary(
            "page",
            "Not saved",
            "you need Manage Server in this server to change this"
        )),
        "{}",
        refused.html
    );
    assert!(
        refused
            .html
            .contains(&load_error("Couldn't load the reaction roles", FORBIDDEN)),
        "{}",
        refused.html
    );
    assert_eq!(app.mappings(7).await.unwrap().len(), 2);

    app.pool.close().await;
}

const GR: &str = "/guild/7/greetings";

fn action(name: &str) -> String {
    format!("{GR}/{name}")
}

const CHANNEL_SELECT_OPTIONS: &str = "<option value=\"\" selected=\"\">(not set)</option><option value=\"23\">\u{25b8} Lounge</option><option value=\"20\"># rules</option><option value=\"21\"># chat</option><option value=\"22\"># bots</option>";
const LEGEND_LIST: &str = r#"<ul class="greet-legend"><li><code>{user}</code> - mentions the person being greeted, or the sender when the command is run without a user.</li><li><code>{author}</code> - mentions whoever ran the command.</li></ul>"#;
const SEE_PLANS: &str = r#"<p class="plan-note"><span class="plan-tag">Pro</span><span>On Pro these floors drop to 3s and 1s.</span><a href="/upgrade">See plans</a></p>"#;
const GOOD_WORKS_LEAD: &str = "<h2 class=\"label\" id=\"good-channels-title\">Where /good works</h2><p class=\"page-lead\">With nothing listed, <code>/good</code> works in every channel. Add one or more and Discord hides the command everywhere else - it never even shows up in the picker. Adding a category covers every channel inside it.</p><p class=\"page-lead\">This writes the same command permissions as Discord's own Server Settings \u{2192} Integrations panel, so changes made either way show up in both.</p>";
const NO_IMAGES: &str = r#"<p class="page-lead">No images yet - the command replies with just the message until you add one.</p>"#;
const NOT_SYNCED_HEADER: &str = r#"<p class="field-hint">Not synced yet: this module's state appears once Zayden is in the server and has synced its commands.</p></div><div class="settings-actions"><span class="lamp-status"><span class="lamp lamp-sync" aria-hidden="true"></span><span class="lamp-text">Not synced</span></span></div></div>"#;

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

/// The `value` attribute of the input with id `id`.
fn value<'a>(html: &'a str, id: &str) -> Option<&'a str> {
    element(html, "input", id)?
        .split_once(r#" value=""#)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(value, _)| value)
}

/// The inline error of field `id`.
fn field_error<'a>(html: &'a str, id: &str) -> Option<&'a str> {
    let (_, rest) =
        html.split_once(&format!(r#"<p class="field-error" id="{id}-error">"#))?;
    rest.split_once("</p>").map(|(text, _)| text)
}

/// The result line in the save bar of the form in section `section`.
fn bar_flash<'a>(html: &'a str, section: &str) -> Option<&'a str> {
    let section = element(html, "section", section)?;
    let (_, bar) = section.split_once(r#"<div class="save-bar""#)?;
    let (_, rest) = bar.split_once(r#"<span class="flash-text">"#)?;
    rest.split_once("</span>").map(|(text, _)| text)
}

/// The result line of section `section` outside a save bar.
fn section_flash<'a>(html: &'a str, section: &str) -> Option<&'a str> {
    let section = element(html, "section", section)?;
    let (_, region) =
        section.split_once(r#"<div class="flash-region" role="status">"#)?;
    let (_, rest) = region.split_once(r#"<span class="flash-text">"#)?;
    rest.split_once("</span>").map(|(text, _)| text)
}

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
    assert!(
        html.contains(&format!(
            r#"<div class="page-header"><div><h1>Greetings</h1><p class="page-lead">What Zayden posts for <code>/good morning</code> and <code>/good night</code>. Each subcommand replies with one image picked at random from its list, plus the message below if you set one.</p>{NOT_SYNCED_HEADER}"#
        )),
        "{html}"
    );
    assert!(!html.contains(r#"role="switch""#), "{html}");
    assert!(
        html.contains(&format!(
            r#"<section class="settings-section" id="greeting-messages" aria-labelledby="greeting-messages-title"><h2 class="label" id="greeting-messages-title">Messages</h2><form method="post" action="{}" data-pending="" data-dirty-guard=""><input type="hidden" name="guild" value="7">"#,
            action("save-messages")
        )),
        "{html}"
    );
    assert!(
        html.contains(
            r#"<label class="field-label" for="greeting-messages-morning-message">Good morning message</label><input class="input" id="greeting-messages-morning-message" type="text" name="morning_message" value="" autocomplete="off" aria-describedby="greeting-messages-morning-message-help"><p class="field-help" id="greeting-messages-morning-message-help">Up to 1500 characters. Leave blank to post just the image.</p>"#
        ),
        "{html}"
    );
    assert_eq!(value(&html, "greeting-messages-night-message"), Some(""));
    assert!(
        html.contains(&format!(
            r#"{LEGEND_LIST}<div class="save-bar" data-save-bar="">"#
        )),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            r#"{GOOD_WORKS_LEAD}<div class="flash-region" role="status"></div><p class="page-lead">No channels listed: <code>/good</code> works everywhere.</p><form method="post" action="{}" data-pending="" data-dirty-guard=""><input type="hidden" name="guild" value="7"><div class="field-row"><label class="field-label" for="add-good-channel-channel-id">Allow a channel</label><div class="select"><select class="input" id="add-good-channel-channel-id" name="channel_id" required="" aria-describedby="add-good-channel-channel-id-help">{CHANNEL_SELECT_OPTIONS}</select>"#,
            action("add-channel")
        )),
        "{html}"
    );
    assert!(
        html.contains(
            r#"<input class="input" id="greeting-cooldowns-user-cooldown" type="number" name="user_cooldown" value="15" min="15" max="86400" step="1" autocomplete="off" aria-describedby="greeting-cooldowns-user-cooldown-help"><p class="field-help" id="greeting-cooldowns-user-cooldown-help">In seconds: at least 15 on the Free plan, at most 86400. Leave blank for the minimum.</p>"#
        ),
        "{html}"
    );
    assert!(
        html.contains(
            r#"<input class="input" id="greeting-cooldowns-guild-cooldown" type="number" name="guild_cooldown" value="3" min="3" max="86400" step="1" autocomplete="off" aria-describedby="greeting-cooldowns-guild-cooldown-help"><p class="field-help" id="greeting-cooldowns-guild-cooldown-help">In seconds: at least 3 on the Free plan, at most 86400. Leave blank for the minimum.</p>"#
        ),
        "{html}"
    );
    assert!(
        html.contains(&format!(r#"{SEE_PLANS}<div class="save-bar""#)),
        "{html}"
    );
    for kind in ["morning", "night"] {
        assert!(
            html.contains(&format!(
                r#"<span class="mono">0 of 50</span></h3><form method="post" action="{}" data-pending="" data-dirty-guard=""><input type="hidden" name="guild" value="7"><input type="hidden" name="kind" value="{kind}"><div class="field-row"><label class="field-label" for="add-{kind}-image-url">Image link</label><input class="input" id="add-{kind}-image-url" type="url" name="url" value="" pattern="https://.*" placeholder="https://example.com/sunrise.gif" required="" maxlength="2048""#,
                action("add-image")
            )),
            "{kind}: {html}"
        );
    }
    assert_eq!(count(&html, NO_IMAGES), 2, "{html}");
    assert_eq!(count(&html, r#"role="alert""#), 0, "{html}");
    assert_eq!(count(&html, "flash-text"), 0, "{html}");

    assert_eq!(app.app.entitlements.guild_tier(7).await, Tier::Free);

    for query in ["channel_added=1", "channel_removed=1", "image_added=1", "saved=1"]
    {
        let page = app.page(&format!("/guild/7/greetings?{query}")).await.unwrap();
        assert_eq!(count(&page, "flash-text"), 0, "{query}");
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
    assert!(pro.contains("In seconds: at least 3 on the Pro plan"), "{pro}");
    assert!(pro.contains("In seconds: at least 1 on the Pro plan"), "{pro}");
    assert!(pro.contains(r#"name="user_cooldown" value="15" min="3""#), "{pro}");
    assert!(pro.contains(r#"name="guild_cooldown" value="3" min="1""#), "{pro}");
    assert!(!pro.contains("See plans"), "{pro}");
    assert!(!pro.contains("these floors drop"), "{pro}");
    let below = app
        .post(
            "/guild/12/greetings/save-cooldowns",
            &[("guild", "12"), ("user_cooldown", "2"), ("guild_cooldown", "1")],
            Some(ADMIN),
        )
        .await
        .unwrap();
    assert_eq!(below.status, StatusCode::UNPROCESSABLE_ENTITY);
    let floor = "On the Pro plan the per-member cooldown can't go below 3s. That is as low as this command goes.";
    assert!(
        below.html.contains(&summary("greeting-cooldowns", "Not saved", floor)),
        "{}",
        below.html
    );
    assert_eq!(
        field_error(&below.html, "greeting-cooldowns-user-cooldown"),
        Some(floor)
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
            r#"<p class="warning">Discord didn't report which channels <code>/good</code> is allowed in, so its restrictions can't be shown or changed right now.</p></section>"#
        ),
        "{unknown}"
    );
    assert!(!unknown.contains("With nothing listed"), "{unknown}");
    assert!(!unknown.contains(&action("add-channel")), "{unknown}");
    assert!(
        unknown.contains("This writes the same command permissions"),
        "{unknown}"
    );

    let member = app.get(GR, Some(MEMBER)).await.unwrap();
    assert_eq!(member.status, StatusCode::OK);
    assert!(
        member.html.contains(&load_error("Couldn't load the greetings", FORBIDDEN)),
        "{}",
        member.html
    );
    assert!(!member.html.contains("greeting-messages"), "{}", member.html);
    assert!(login_redirect(&app.get(GR, None).await.unwrap()));
    let malformed = app.get("/guild/abc/greetings", Some(ADMIN)).await.unwrap();
    assert!(
        malformed
            .html
            .contains(&load_error("Couldn't load the greetings", NOT_A_SERVER)),
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

fn image_card(url: &str, id: &str, host: &str, noun: &str) -> String {
    format!(
        r#"<li class="greet-card"><img class="greet-thumb" src="{url}" alt="Image from {host}" loading="lazy"><a class="greet-url" href="{url}" rel="external noreferrer" target="_blank" title="{url}">{url}</a><form class="greet-remove" method="post" action="{}" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="id" value="{id}"><div class="confirm" data-confirm=""><button type="submit" class="btn btn-ghost" data-confirm-trigger="">Remove</button><dialog class="dialog" aria-labelledby="image-{id}-remove-title" aria-describedby="image-{id}-remove-desc" data-confirm-dialog=""><div class="dialog-panel"><h2 class="dialog-title" id="image-{id}-remove-title">Remove this {noun} image from {host}?</h2><p class="dialog-desc" id="image-{id}-remove-desc">{url}</p>"#,
        action("remove-image")
    )
}

#[sqlx::test(migrations = "../migrations")]
async fn the_greetings_saves_redirect_with_their_result_or_rerender_with_the_reason(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();
    app.mock_free_guild(7);
    app.guild_directory(7);

    let saved = submit(&app, "save-messages", vec![
        ("morning_message", "  Good morning {user}!  "),
        ("night_message", "Night, {author}"),
    ])
    .await
    .unwrap();
    assert_eq!(saved.status, StatusCode::SEE_OTHER);
    assert_eq!(
        saved.location.as_deref(),
        Some("/guild/7/greetings#greeting-messages")
    );
    let page = app.follow(&saved).await.unwrap();
    assert!(page.html.contains(&title(GREETINGS_TITLE)), "{}", page.html);
    assert_eq!(
        bar_flash(&page.html, "greeting-messages"),
        Some("Greeting messages saved.")
    );
    assert_eq!(count(&page.html, "flash-text"), 1, "{}", page.html);
    assert_eq!(
        value(&page.html, "greeting-messages-morning-message"),
        Some("Good morning {user}!")
    );
    assert_eq!(
        value(&page.html, "greeting-messages-night-message"),
        Some("Night, {author}")
    );
    assert_eq!(count(&page.html, r#"role="alert""#), 0, "{}", page.html);

    let too_long = "x".repeat(1501);
    let refused = submit(&app, "save-messages", vec![
        ("morning_message", &too_long),
        ("night_message", "kept?"),
    ])
    .await
    .unwrap();
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(refused.html.contains(&title(GREETINGS_TITLE)), "{}", refused.html);
    assert!(
        refused.html.contains(&summary(
            "greeting-messages",
            "Not saved",
            "Greeting messages are limited to 1500 characters so the reply still fits once mentions are filled in."
        )),
        "{}",
        refused.html
    );
    assert_eq!(
        value(&refused.html, "greeting-messages-morning-message"),
        Some(too_long.as_str())
    );
    assert_eq!(
        value(&refused.html, "greeting-messages-night-message"),
        Some("kept?")
    );

    let cooldowns = submit(&app, "save-cooldowns", vec![
        ("user_cooldown", "30"),
        ("guild_cooldown", " 10 "),
    ])
    .await
    .unwrap();
    assert_eq!(cooldowns.status, StatusCode::SEE_OTHER);
    assert_eq!(
        cooldowns.location.as_deref(),
        Some("/guild/7/greetings#greeting-cooldowns")
    );
    let page = app.follow(&cooldowns).await.unwrap();
    assert_eq!(
        bar_flash(&page.html, "greeting-cooldowns"),
        Some("Cooldowns saved.")
    );
    assert_eq!(value(&page.html, "greeting-cooldowns-user-cooldown"), Some("30"));
    assert_eq!(value(&page.html, "greeting-cooldowns-guild-cooldown"), Some("10"));
    assert!(page.html.contains(SEE_PLANS), "{}", page.html);

    let below = submit(&app, "save-cooldowns", vec![
        ("user_cooldown", "5"),
        ("guild_cooldown", ""),
    ])
    .await
    .unwrap();
    assert_eq!(below.status, StatusCode::UNPROCESSABLE_ENTITY);
    let floor = "On the Free plan the per-member cooldown can't go below 15s. Pro servers can go as low as 3s.";
    assert!(
        below.html.contains(&summary("greeting-cooldowns", "Not saved", floor)),
        "{}",
        below.html
    );
    assert_eq!(
        field_error(&below.html, "greeting-cooldowns-user-cooldown"),
        Some(floor)
    );
    assert_eq!(value(&below.html, "greeting-cooldowns-user-cooldown"), Some("5"));
    assert_eq!(value(&below.html, "greeting-cooldowns-guild-cooldown"), Some(""));
    let junk = submit(&app, "save-cooldowns", vec![
        ("user_cooldown", "abc"),
        ("guild_cooldown", "3"),
    ])
    .await
    .unwrap();
    assert!(
        junk.html.contains(&summary(
            "greeting-cooldowns",
            "Not saved",
            "`abc` isn't a usable cooldown. Enter a whole number of seconds between 0 and 86400."
        )),
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
        Some("/guild/7/greetings#greeting-images")
    );
    let added = app.follow(&created).await.unwrap();
    assert_eq!(added.status, StatusCode::OK);
    assert_eq!(app.images(7, "morning").await.unwrap(), [first]);
    assert_eq!(section_flash(&added.html, "greeting-images"), Some("Image added."));
    let id =
        attribute_after(&added.html, r#"<input type="hidden" name="id" value=""#)
            .expect("image id")
            .to_owned();
    assert!(
        added.html.contains(&format!(
            r#"<ul class="greet-grid" aria-labelledby="morning-images-title">{}"#,
            image_card(first, &id, "example.com", "good morning")
        )),
        "{}",
        added.html
    );
    assert!(
        added
            .html
            .contains(r#"Good morning images <span class="mono">1 of 50</span>"#)
    );
    assert_eq!(count(&added.html, NO_IMAGES), 1, "{}", added.html);
    assert_eq!(count(&added.html, r#"class="greet-grid""#), 1, "{}", added.html);

    let duplicate =
        submit(&app, "add-image", vec![("kind", "morning"), ("url", first)])
            .await
            .unwrap();
    assert_eq!(duplicate.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        duplicate.html.contains(&summary(
            "add-morning-image",
            "Not added",
            "That image link is already in the list."
        )),
        "{}",
        duplicate.html
    );
    let insecure = submit(&app, "add-image", vec![
        ("kind", "night"),
        ("url", "http://example.com/a.gif"),
    ])
    .await
    .unwrap();
    assert!(
        insecure.html.contains(&summary(
            "add-night-image",
            "Not added",
            "`http://example.com/a.gif` isn't a usable image link. Links must start with `https://`."
        )),
        "{}",
        insecure.html
    );
    let unknown_kind =
        submit(&app, "add-image", vec![("kind", "noon"), ("url", first)])
            .await
            .unwrap();
    assert!(
        unknown_kind.html.contains(&summary(
            "add-morning-image",
            "Not added",
            "Unknown greeting type `noon`."
        )),
        "{}",
        unknown_kind.html
    );
    assert_eq!(app.images(7, "night").await.unwrap().len(), 0);

    let missing =
        submit(&app, "remove-image", vec![("id", "999999")]).await.unwrap();
    assert_eq!(missing.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        missing.html.contains(&summary(
            "remove-greeting-image",
            "Not removed",
            "that image is not in this server's list"
        )),
        "{}",
        missing.html
    );
    let done = submit(&app, "remove-image", vec![("id", &id)]).await.unwrap();
    assert_eq!(done.status, StatusCode::SEE_OTHER);
    assert_eq!(done.location.as_deref(), Some("/guild/7/greetings#greeting-images"));
    let removed = app.follow(&done).await.unwrap();
    assert_eq!(removed.status, StatusCode::OK);
    assert_eq!(app.images(7, "morning").await.unwrap().len(), 0);
    assert_eq!(
        section_flash(&removed.html, "greeting-images"),
        Some("Image removed.")
    );
    assert_eq!(count(&removed.html, NO_IMAGES), 2, "{}", removed.html);

    let unregistered =
        submit(&app, "add-channel", vec![("channel_id", "20")]).await.unwrap();
    assert_eq!(unregistered.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        unregistered.html.contains(&summary(
            "add-good-channel",
            "Not added",
            "/good isn't registered for this server yet"
        )),
        "{}",
        unregistered.html
    );
    let foreign =
        submit(&app, "add-channel", vec![("channel_id", "99")]).await.unwrap();
    assert!(
        foreign.html.contains("Not added: that channel is not in this server"),
        "{}",
        foreign.html
    );
    let removed =
        submit(&app, "remove-channel", vec![("channel_id", "20")]).await.unwrap();
    assert!(
        removed.html.contains(&summary(
            "remove-good-channel",
            "Not removed",
            "/good isn't registered for this server yet"
        )),
        "{}",
        removed.html
    );

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
        missing.html.contains(&summary(
            "greeting-messages",
            "Not saved",
            "missing field `night_message`"
        )),
        "{}",
        missing.html
    );
    assert_eq!(
        field_error(&missing.html, "greeting-messages-night-message"),
        Some("missing field `night_message`")
    );
    let aimed = app
        .post(&action("remove-image"), &[("guild", "11"), ("id", &id)], Some(ADMIN))
        .await
        .unwrap();
    assert_eq!(aimed.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(writes(&app), sent);

    let unknown_action =
        app.post(&action("reset"), &[("guild", "7")], Some(ADMIN)).await.unwrap();
    assert_eq!(unknown_action.status, StatusCode::NOT_FOUND);
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
    assert!(
        member.html.contains(&load_error("Couldn't load the greetings", FORBIDDEN)),
        "{}",
        member.html
    );
    assert_eq!(app.images(7, "morning").await.unwrap().len(), 0);

    app.pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn the_greetings_header_switch_turns_the_module_on_and_off(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();
    app.mock_free_guild(7);
    app.guild_directory(7);
    app.app.modules.set(7, "greetings", true).await.unwrap();
    assert_eq!(
        app.app.modules.states(7).await.unwrap().get("greetings").copied(),
        Some(true)
    );

    let html = app.page(GR).await.unwrap();
    assert!(
        html.contains(&format!(
            r#"<form class="settings-actions" method="post" action="{}" data-pending=""><input type="hidden" name="guild" value="7"><span class="lamp-status"><span class="lamp lamp-on" aria-hidden="true"></span><span class="lamp-text" data-pending-text="Saving…">On</span></span><button type="submit" class="switch" role="switch" aria-checked="true" aria-label="Greetings module" name="enabled" value="false">"#,
            action("module")
        )),
        "{html}"
    );
    assert!(!html.contains("Not synced"), "{html}");

    let off = submit(&app, "module", vec![("enabled", "false")]).await.unwrap();
    assert_eq!(off.status, StatusCode::SEE_OTHER);
    assert_eq!(off.location.as_deref(), Some(GR));
    assert_eq!(
        app.app.modules.states(7).await.unwrap().get("greetings").copied(),
        Some(false)
    );
    let page = app.follow(&off).await.unwrap();
    assert!(
        page.html.contains(&format!(
            r#"</div><div class="flash-region" role="status"><p class="flash flash-success" data-flash="">{CHECK}<span class="flash-text">Greetings turned off.</span></p></div><section class="settings-section" id="greeting-messages""#
        )),
        "{}",
        page.html
    );
    assert!(page.html.contains(r#"aria-checked="false" aria-label="Greetings module" name="enabled" value="true""#), "{}", page.html);

    let refused = submit(&app, "module", vec![("enabled", "maybe")]).await.unwrap();
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        refused.html.contains(r#"<p class="error" role="alert">Not changed: invalid value for `enabled`</p>"#),
        "{}",
        refused.html
    );
    assert!(refused.html.contains(&title(GREETINGS_TITLE)), "{}", refused.html);
    assert_eq!(
        app.app.modules.states(7).await.unwrap().get("greetings").copied(),
        Some(false)
    );

    app.pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn a_refused_greetings_save_keeps_what_was_typed_in_its_own_form_only(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();
    app.mock_free_guild(7);
    app.guild_directory(7);

    let low = submit(&app, "save-cooldowns", vec![
        ("user_cooldown", "1"),
        ("guild_cooldown", "1"),
    ])
    .await
    .unwrap();
    assert_eq!(low.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        low.html.contains(&summary(
            "greeting-cooldowns",
            "Not saved",
            "On the Free plan the per-member cooldown can't go below 15s. Pro servers can go as low as 3s."
        )),
        "{}",
        low.html
    );
    assert_eq!(value(&low.html, "greeting-cooldowns-user-cooldown"), Some("1"));
    assert_eq!(value(&low.html, "greeting-cooldowns-guild-cooldown"), Some("1"));
    assert_eq!(value(&low.html, "greeting-messages-morning-message"), Some(""));
    assert_eq!(value(&low.html, "greeting-messages-night-message"), Some(""));

    let sibling = submit(&app, "save-messages", vec![
        ("morning_message", &"x".repeat(1501)),
        ("night_message", "typed night"),
    ])
    .await
    .unwrap();
    assert_eq!(sibling.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        value(&sibling.html, "greeting-messages-night-message"),
        Some("typed night")
    );
    assert_eq!(value(&sibling.html, "greeting-cooldowns-user-cooldown"), Some("15"));
    assert_eq!(value(&sibling.html, "greeting-cooldowns-guild-cooldown"), Some("3"));
    assert_eq!(count(&sibling.html, r#"role="alert""#), 1, "{}", sibling.html);

    let insecure = submit(&app, "add-image", vec![
        ("kind", "night"),
        ("url", "http://example.com/a.gif"),
    ])
    .await
    .unwrap();
    assert_eq!(insecure.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        value(&insecure.html, "add-night-image-url"),
        Some("http://example.com/a.gif")
    );
    assert_eq!(value(&insecure.html, "add-morning-image-url"), Some(""));

    let unregistered =
        submit(&app, "add-channel", vec![("channel_id", "21")]).await.unwrap();
    assert_eq!(unregistered.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        unregistered
            .html
            .contains(r#"<option value="21" selected=""># chat</option>"#),
        "{}",
        unregistered.html
    );

    app.pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn a_refused_reaction_role_add_keeps_what_was_typed(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    let app = harness(pool.clone(), |base| base).await.unwrap();
    app.guild_directory(7);

    let refused = app
        .post(
            RR_ADD,
            &[
                ("guild", "7"),
                ("channel_id", "21"),
                ("message_id", "4242"),
                ("role_id", "31"),
                ("emoji", "\u{1f514}"),
            ],
            Some(ADMIN),
        )
        .await
        .unwrap();
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        refused.html.contains(r#"<div class="error" id="add-reaction-role-summary" role="alert" tabindex="-1" autofocus="">Not added: "#),
        "{}",
        refused.html
    );
    for typed in [
        r#"<option value="21" selected=""># chat</option>"#,
        r#"<option value="31" selected="">@Mod</option>"#,
        r#"name="message_id" value="4242""#,
        "name=\"emoji\" value=\"\u{1f514}\"",
    ] {
        assert_eq!(count(&refused.html, typed), 1, "{typed}: {}", refused.html);
    }

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
            r#"<thead role="rowgroup"><tr role="row"><th scope="col" role="columnheader">Channel</th></tr></thead><tbody role="rowgroup"><tr role="row"><th scope="row" role="rowheader" data-label="Channel">#chat</th></tr><tr role="row"><th scope="row" role="rowheader" data-label="Channel">#bots</th></tr></tbody></table></div><p class="warning">Read-only: Discord only lets a member with Manage Server change which channels a command is allowed in.</p></section>"#
        ),
        "{html}"
    );
    assert!(html.contains("With nothing listed"), "{html}");
    assert!(!html.contains("/guild/8/greetings/add-channel"), "{html}");
    assert!(!html.contains("/guild/8/greetings/remove-channel"), "{html}");
    assert!(
        html.contains(
            r#"<form method="post" action="/guild/8/greetings/save-messages""#
        ),
        "{html}"
    );

    let refused = app
        .post(
            "/guild/8/greetings/add-channel",
            &[("guild", "8"), ("channel_id", "20")],
            Some(OPERATOR),
        )
        .await
        .unwrap();
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, reason) = refused
        .html
        .split_once(r#"<div class="error" id="add-good-channel-summary" role="alert" tabindex="-1" autofocus="">Not added: "#)
        .unwrap_or_else(|| panic!("{}", refused.html));
    assert!(
        reason.contains(r#"</div><p class="warning">Read-only: "#),
        "{}",
        refused.html
    );
    assert!(
        !refused.html.contains("error running server function"),
        "{}",
        refused.html
    );
    assert_eq!(app.discord.count("PUT", "/"), 0);

    let admin = app.get("/guild/8/greetings", Some(ADMIN)).await.unwrap();
    assert!(
        admin.html.contains(&load_error("Couldn't load the greetings", FORBIDDEN)),
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
    let failed = (variant != "editable").then_some("boom");

    Ok(view! {
        channel_section(
            guild_id: "7",
            allowed: (variant != "unknown").then_some(allowed.as_slice()),
            channels: &channels,
            locked: variant == "locked",
            add_error: failed,
            add_field_error: failed,
            remove_error: (variant == "editable").then_some("gone")
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
async fn the_channel_section_lists_removable_channels_for_a_member() {
    let html = standalone("/test/channels/editable").await.unwrap();
    let head = r#"<caption class="visually-hidden">Channels where /good works</caption><thead role="rowgroup"><tr role="row"><th scope="col" role="columnheader">Channel</th><th scope="col" role="columnheader">Action</th></tr></thead>"#;
    assert!(html.contains(head), "{html}");
    for (index, id, label) in [(0, "21", "#chat"), (1, "99", "#unknown (99)")] {
        assert!(
            html.contains(&format!(
                r#"<tr role="row"><th scope="row" role="rowheader" data-label="Channel">{label}</th><td role="cell" data-label="Action"><form method="post" action="/guild/7/greetings/remove-channel" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="channel_id" value="{id}"><div class="confirm" data-confirm=""><button type="submit" class="btn btn-ghost" data-confirm-trigger="">Remove</button><dialog class="dialog" aria-labelledby="good-channel-{index}-remove-title" aria-describedby="good-channel-{index}-remove-desc" data-confirm-dialog=""><div class="dialog-panel"><h2 class="dialog-title" id="good-channel-{index}-remove-title">Remove {label} from the /good list?</h2><p class="dialog-desc" id="good-channel-{index}-remove-desc">Removing the last channel lets /good work everywhere again.</p><div class="dialog-actions"><button type="button" class="btn btn-secondary" data-dialog-close="" autofocus="">Cancel</button><button type="submit" class="btn btn-danger">Remove channel</button></div>"#
            )),
            "{label}: {html}"
        );
    }
    let removed_at = html
        .find(r#"<div class="error" id="remove-good-channel-summary" role="alert" tabindex="-1" autofocus="">Not removed: gone</div>"#)
        .unwrap();
    assert!(removed_at < html.find("data-table-wrap").unwrap(), "{html}");
    assert!(!html.contains("Not added"), "{html}");
    assert!(
        html.contains(r#"<option value="" selected="">(not set)</option><option value="22"># bots</option></select>"#)
            && !html.contains(r#"<option value="21">"#),
        "{html}"
    );
    assert!(html.contains(r#"<form method="post" action="/guild/7/greetings/add-channel" data-pending="" data-dirty-guard="">"#), "{html}");

    let locked = standalone("/test/channels/locked").await.unwrap();
    assert!(
        locked.contains(
            r#"<thead role="rowgroup"><tr role="row"><th scope="col" role="columnheader">Channel</th></tr></thead><tbody role="rowgroup"><tr role="row"><th scope="row" role="rowheader" data-label="Channel">#chat</th></tr><tr role="row"><th scope="row" role="rowheader" data-label="Channel">#unknown (99)</th></tr></tbody></table></div><div class="error" id="add-good-channel-summary" role="alert" tabindex="-1" autofocus="">Not added: boom</div><p class="warning">Read-only: Discord only lets a member with Manage Server change which channels a command is allowed in.</p></section>"#
        ),
        "{locked}"
    );
    assert!(!locked.contains("remove-channel"), "{locked}");
    assert!(!locked.contains("add-channel"), "{locked}");

    let unknown = standalone("/test/channels/unknown").await.unwrap();
    assert!(
        unknown.contains(
            r#"<div class="flash-region" role="status"></div><div class="error" id="add-good-channel-summary" role="alert" tabindex="-1" autofocus="">Not added: boom</div><p class="warning">Discord didn't report which channels <code>/good</code> is allowed in, so its restrictions can't be shown or changed right now.</p></section>"#
        ),
        "{unknown}"
    );
    assert!(!unknown.contains("With nothing listed"), "{unknown}");
    assert!(!unknown.contains("data-table"), "{unknown}");
}

#[tokio::test]
async fn an_unreachable_database_leaves_each_page_with_its_load_error() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy("postgres://127.0.0.1:1/unused")
        .unwrap();
    let app = harness_without_database(pool).unwrap();

    for (path, title) in [
        ("/guild/7/levels", "Couldn't load the leaderboard"),
        ("/guild/7/reaction-roles", "Couldn't load the reaction roles"),
        ("/guild/7/greetings", "Couldn't load the greetings"),
    ] {
        let reply = app.get(path, Some("session=uncached-token")).await.unwrap();
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        let panel = format!(
            r#"<section class="error-panel" role="alert" aria-labelledby="load-error-title"><h2 class="error-title" id="load-error-title">{title}</h2><p class="error-text">Something went wrong: "#
        );
        assert!(reply.html.contains(&panel), "{path}: {}", reply.html);
        assert!(
            reply.html.contains(&format!(
                r#"<div class="error-actions"><a href="{path}" class="btn btn-primary">Try again</a><a href="/guilds" class="btn btn-secondary">Back to servers</a></div></section>"#
            )),
            "{path}: {}",
            reply.html
        );
        assert!(!reply.html.contains("error running server function"), "{path}");
        assert!(!reply.html.contains("Add a mapping"), "{path}");
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
