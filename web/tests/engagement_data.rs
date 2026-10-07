//! Engagement loaders and saves against Postgres and a stand-in Discord API,
//! driven through the app router so each call authorizes from a real `session`
//! cookie. The bot's Discord client is pointed at a local server that answers
//! the few endpoints each case needs and records every request it gets, so a
//! case can also assert what was never sent.

use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use http_body_util::BodyExt;
use jiff::{SignedDuration, Timestamp};
use sqlx::PgPool;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, header, path_param, route};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use url::form_urlencoded;
use web::auth::SessionUser;
use web::engagement::greetings::{
    AddGreetingImageForm,
    GreetingChannelForm,
    RemoveGreetingImageForm,
    SaveGreetingCooldownsForm,
    SaveGreetingMessagesForm,
    add_greeting_channel,
    add_greeting_image,
    get_greetings,
    load_greetings_page,
    remove_greeting_channel,
    remove_greeting_image,
    save_greeting_cooldowns,
    save_greeting_messages,
};
use web::engagement::levels::get_leaderboard;
use web::engagement::reaction_roles::{
    AddReactionRoleForm,
    RemoveReactionRoleForm,
    add_reaction_role,
    list_reaction_roles,
    load_reaction_roles_page,
    remove_reaction_role,
};
use web::engagement::{
    CooldownView,
    EngagementError,
    GreetingImageInfo,
    GreetingsView,
    LeaderboardView,
};
use web::guild::dto::Tier as Plan;
use web::state::{DiscordState, SessionUsersCache, UserGuildsCache, WebState};
use web::util::server_error_text;
use zayden_app::config::BotConfig;
use zayden_app::entitlement::{EntitlementScope, Tier};
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
type Fields = Vec<(&'static str, &'static str)>;

const ADMIN: &str = "session=admin-token";
const MEMBER: &str = "session=member-token";
const OPERATOR: &str = "session=operator-token";
const APP_ID: u64 = 123_456_789;
const AVATAR: &str = "abcdef0123456789abcdef0123456789";

path_param!(guild);
path_param!(action);

#[derive(Clone, Debug)]
struct Hit {
    method: String,
    path: String,
    body: String,
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
        let body = String::from_utf8_lossy(&body).into_owned();

        let path = target
            .split('?')
            .next()
            .unwrap_or_default()
            .trim_start_matches("/api/v10")
            .to_owned();
        let (status, reply) = self.answer(&method, &path);
        locked(&self.hits).push(Hit { method, path, body });

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

    fn total(&self) -> usize {
        locked(&self.hits).len()
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

fn debug<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

fn shown(message: &str) -> String {
    server_error_text(message)
}

fn render<T>(
    result: std::result::Result<T, EngagementError>,
    ok: impl FnOnce(T) -> String,
) -> String {
    result.map_or_else(server_error_text, ok)
}

fn done(result: std::result::Result<(), EngagementError>) -> String {
    render(result, |()| "ok".to_owned())
}

#[route(GET "/test/levels/{guild}")]
async fn levels(cx: &Cx) -> Result<String> {
    let guild: &str = path_param::<Guild>(cx);
    let view = LeaderboardView::from_request(cx);

    Ok(render(get_leaderboard(cx, guild, view.global, view.page).await, |page| {
        let entries = page
            .entries
            .iter()
            .map(|e| {
                format!(
                    "{}|{}|{}|{}|{}|{}|{}",
                    e.rank,
                    e.user_id,
                    e.name,
                    e.avatar.as_deref().unwrap_or("-"),
                    e.level,
                    e.xp,
                    e.message_count
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        format!("next={};{entries}", page.has_next)
    }))
}

#[route(GET "/test/levels-raw/{guild}/{action}")]
async fn levels_raw(cx: &Cx) -> Result<String> {
    let guild: &str = path_param::<Guild>(cx);
    let page: i32 = path_param::<Action>(cx).parse().unwrap_or(1);

    Ok(render(get_leaderboard(cx, guild, false, page).await, |page| {
        format!(
            "{} next={}",
            page.entries
                .iter()
                .map(|e| e.rank.to_string())
                .collect::<Vec<_>>()
                .join(","),
            page.has_next
        )
    }))
}

#[route(GET "/test/rr/list/{guild}")]
async fn rr_list(cx: &Cx) -> Result<String> {
    let guild: &str = path_param::<Guild>(cx);
    Ok(render(list_reaction_roles(cx, guild).await, |rows| {
        rows.iter()
            .map(|r| {
                format!(
                    "{}/{}/{}/{}",
                    r.channel_id, r.message_id, r.role_id, r.emoji
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }))
}

#[route(GET "/test/rr/page/{guild}")]
async fn rr_page(cx: &Cx) -> Result<String> {
    let guild: &str = path_param::<Guild>(cx);
    Ok(render(load_reaction_roles_page(cx, guild).await, |page| {
        let channels = page
            .channels
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let roles =
            page.roles.iter().map(|r| r.name.as_str()).collect::<Vec<_>>().join(",");
        format!(
            "{} mappings; channels {channels}; roles {roles}",
            page.mappings.len()
        )
    }))
}

#[route(POST "/test/rr/{action}")]
async fn rr_post(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<String> {
    let action: &str = path_param::<Action>(cx);
    Ok(match action {
        "add" => match AddReactionRoleForm::from_pairs(pairs) {
            Ok(form) => done(add_reaction_role(cx, &form).await),
            Err(e) => shown(&e.to_string()),
        },
        "remove" => match RemoveReactionRoleForm::from_pairs(pairs) {
            Ok(form) => done(remove_reaction_role(cx, &form).await),
            Err(e) => shown(&e.to_string()),
        },
        other => other.to_owned(),
    })
}

#[route(GET "/test/greetings-view/{guild}")]
async fn greetings_view(cx: &Cx) -> Result<String> {
    let guild: &str = path_param::<Guild>(cx);
    Ok(render(get_greetings(cx, guild).await, |view| debug(&view)))
}

#[route(GET "/test/greetings-page/{guild}")]
async fn greetings_page(cx: &Cx) -> Result<String> {
    let guild: &str = path_param::<Guild>(cx);
    Ok(render(load_greetings_page(cx, guild).await, |page| {
        format!(
            "{} channels: {}",
            page.channels.len(),
            page.channels
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(",")
        )
    }))
}

#[route(POST "/test/greetings/{action}")]
async fn greetings_post(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<String> {
    let action: &str = path_param::<Action>(cx);
    let result = match action {
        "messages" => match SaveGreetingMessagesForm::from_pairs(pairs) {
            Ok(form) => save_greeting_messages(cx, &form).await,
            Err(e) => Err(e),
        },
        "cooldowns" => match SaveGreetingCooldownsForm::from_pairs(pairs) {
            Ok(form) => save_greeting_cooldowns(cx, &form).await,
            Err(e) => Err(e),
        },
        "add-channel" => match GreetingChannelForm::from_pairs(pairs) {
            Ok(form) => add_greeting_channel(cx, &form).await,
            Err(e) => Err(e),
        },
        "remove-channel" => match GreetingChannelForm::from_pairs(pairs) {
            Ok(form) => remove_greeting_channel(cx, &form).await,
            Err(e) => Err(e),
        },
        "add-image" => match AddGreetingImageForm::from_pairs(pairs) {
            Ok(form) => add_greeting_image(cx, &form).await,
            Err(e) => Err(e),
        },
        "remove-image" => match RemoveGreetingImageForm::from_pairs(pairs) {
            Ok(form) => remove_greeting_image(cx, &form).await,
            Err(e) => Err(e),
        },
        other => return Ok(other.to_owned()),
    };

    Ok(done(result))
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

async fn seed_guilds(cache: &UserGuildsCache) {
    cache
        .insert(
            41,
            Arc::from([
                guild(7, "Guild 7", Permissions::MANAGE_GUILD),
                guild(11, "Guild 11", Permissions::MANAGE_GUILD),
                guild(12, "Guild 12", Permissions::ADMINISTRATOR),
            ]),
        )
        .await;
    cache
        .insert(43, Arc::from([guild(7, "Guild 7", Permissions::SEND_MESSAGES)]))
        .await;
    cache.insert(44, Arc::from([])).await;
}

async fn insert_session(pool: &PgPool, token: &str, user_id: i64) -> TestResult<()> {
    let expires_at = jiff_sqlx::Timestamp::from(
        Timestamp::now().checked_add(SignedDuration::from_hours(168))?,
    );

    #[expect(
        trivial_casts,
        reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
    )]
    sqlx::query!(
        "INSERT INTO web_sessions \
             (token, discord_user_id, discord_access_token, expires_at) \
         VALUES ($1, $2, $3, $4)",
        token,
        user_id,
        format!("{token}-access"),
        expires_at as jiff_sqlx::Timestamp
    )
    .execute(pool)
    .await?;
    Ok(())
}

struct Harness {
    router: Router,
    pool: PgPool,
    discord: Discord,
    app: Arc<ZaydenAppState>,
}

async fn harness(pool: PgPool) -> TestResult<Harness> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let discord = Discord::start()?;
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
    seed_guilds(&state.discord.user_guilds).await;
    seed_users(&state.discord.users, &[41, 43, 44]).await;
    insert_session(&pool, "admin-token", 41).await?;
    insert_session(&pool, "member-token", 43).await?;
    insert_session(&pool, "operator-token", 44).await?;
    sqlx::query!(
        "INSERT INTO web_user_roles (discord_user_id, role) VALUES (44, 'operator')"
    )
    .execute(&pool)
    .await?;

    let base = Router::builder()
        .app_context(state)
        .route(levels)
        .route(levels_raw)
        .route(rr_list)
        .route(rr_page)
        .route(rr_post)
        .route(greetings_view)
        .route(greetings_page)
        .route(greetings_post);

    Ok(Harness { router: web::router(base), pool, discord, app })
}

impl Harness {
    async fn get(&self, path: &str, cookie: Option<&str>) -> TestResult<Response> {
        let mut request = Request::get(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        Ok(self.router.handle(request.body(Body::empty())?).await)
    }

    async fn text(&self, path: &str, cookie: Option<&str>) -> TestResult<String> {
        body_text(self.get(path, cookie).await?).await
    }

    async fn admin(&self, path: &str) -> TestResult<String> {
        self.text(path, Some(ADMIN)).await
    }

    async fn post(
        &self,
        path: &str,
        fields: &[(&str, &str)],
        cookie: Option<&str>,
    ) -> TestResult<String> {
        let body = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(fields)
            .finish();
        let mut request = Request::post(path)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let response = self.router.handle(request.body(Body::from(body))?).await;
        body_text(response).await
    }

    async fn admin_post(
        &self,
        path: &str,
        fields: &[(&str, &str)],
    ) -> TestResult<String> {
        self.post(path, fields, Some(ADMIN)).await
    }

    async fn seed_guild(&self, id: i64) -> TestResult<()> {
        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT DO NOTHING",
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_user(&self, id: i64) -> TestResult<()> {
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
    ) -> TestResult<()> {
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
}

async fn body_text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

#[sqlx::test(migrations = "../migrations")]
async fn the_leaderboard_pages_ten_members_and_names_them_from_discord(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
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
    for (user, level, xp, messages) in
        [(1001_i64, 40, 10, 7), (1000, 40, 30, 8), (1002, 3, 1, 9)]
            .into_iter()
            .chain((0..9_i64).map(|k| {
                (
                    1003 + k,
                    1,
                    90 - i32::try_from(k).unwrap_or(0),
                    100 + i32::try_from(k).unwrap_or(0),
                )
            }))
    {
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
    app.discord.get(
        "/users/1003",
        &user_json(1003, "carol", None, Some(&format!("a_{AVATAR}"))),
    );
    app.discord.get("/users/1004", "not json");

    let first = app.admin("/test/levels/7").await.unwrap();
    let rows: Vec<&str> = first.split(';').collect();
    assert_eq!(rows.len(), 11, "{first}");
    assert_eq!(rows[0], "next=true");
    assert_eq!(
        rows[1],
        format!(
            "1|1000|Alice A|https://cdn.discordapp.com/avatars/1000/{AVATAR}.png|20|100|50"
        )
    );
    assert_eq!(rows[2], "2|1001|bob|-|19|99|51");
    assert_eq!(rows[3], "3|1002|User 1002|-|18|98|52");
    assert_eq!(
        rows[4],
        format!(
            "4|1003|carol|https://cdn.discordapp.com/avatars/1003/a_{AVATAR}.png|17|97|53"
        )
    );
    assert_eq!(rows[5], "5|1004|User 1004|-|16|96|54");
    assert_eq!(rows[10], "10|1009|User 1009|-|11|91|59");
    assert_eq!(
        app.discord
            .hits("GET", "/users/")
            .iter()
            .map(|h| h.path.clone())
            .collect::<Vec<_>>(),
        (1000..1010).map(|id| format!("/users/{id}")).collect::<Vec<_>>()
    );
    assert_eq!(app.discord.count("GET", "/users/1010"), 0);

    let second = app.admin("/test/levels/7?page=2").await.unwrap();
    assert_eq!(
        second,
        "next=false;11|1010|User 1010|-|10|90|60;12|1011|User 1011|-|9|89|61"
    );
    assert_eq!(app.admin("/test/levels/7?page=3").await.unwrap(), "next=false;");

    assert_eq!(app.admin("/test/levels/7?page=1&scope=guild").await.unwrap(), first);
    assert_eq!(app.admin("/test/levels/7?page=0").await.unwrap(), first);
    assert_eq!(app.admin("/test/levels/7?page=-4").await.unwrap(), first);
    assert_eq!(app.admin("/test/levels/7?page=x&scope=nope").await.unwrap(), first);
    assert_eq!(
        app.admin("/test/levels-raw/7/-3").await.unwrap(),
        "1,2,3,4,5,6,7,8,9,10 next=true"
    );
    assert_eq!(app.admin("/test/levels-raw/7/2").await.unwrap(), "11,12 next=false");

    let global = app.admin("/test/levels/7?scope=global").await.unwrap();
    let global_rows: Vec<&str> = global.split(';').collect();
    assert_eq!(global_rows.len(), 11, "{global}");
    assert_eq!(global_rows[0], "next=true");
    assert_eq!(
        global_rows[1],
        format!(
            "1|1000|Alice A|https://cdn.discordapp.com/avatars/1000/{AVATAR}.png|40|30|8"
        )
    );
    assert_eq!(global_rows[2], "2|1001|bob|-|40|10|7");
    assert_eq!(global_rows[3], "3|1002|User 1002|-|3|1|9");
    assert_eq!(global_rows[10], "10|1009|User 1009|-|1|84|106");
    assert_eq!(
        app.admin("/test/levels/7?scope=global&page=2").await.unwrap(),
        "next=false;11|1010|User 1010|-|1|83|107;12|1011|User 1011|-|1|82|108"
    );
    assert_eq!(
        app.admin("/test/levels/7?scope=global&page=3").await.unwrap(),
        "next=false;"
    );

    assert_eq!(app.admin("/test/levels/7?page=2&page=3").await.unwrap(), second);
    assert_eq!(
        app.admin("/test/levels/7?scope=global&scope=guild&page=2&page=9")
            .await
            .unwrap(),
        app.admin("/test/levels/7?scope=global&page=2").await.unwrap()
    );
    assert_eq!(app.admin("/test/levels/7?page=%32").await.unwrap(), second);

    assert_eq!(
        app.text("/test/levels/7", Some(MEMBER)).await.unwrap(),
        shown("forbidden")
    );
    assert_eq!(
        app.text("/test/levels/7?scope=global", Some(MEMBER)).await.unwrap(),
        shown("forbidden")
    );
    assert_eq!(
        app.text("/test/levels/7", None).await.unwrap(),
        shown("unauthenticated")
    );
    assert_eq!(
        app.admin("/test/levels/abc").await.unwrap(),
        shown("invalid guild id")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn mappings_list_in_channel_message_emoji_order_for_this_guild_only(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
    app.seed_mapping(7, 30, 5, 1, "\u{1f525}").await.unwrap();
    app.seed_mapping(7, 20, 9, 2, "<:a:5>").await.unwrap();
    app.seed_mapping(7, 20, 9, 3, "\u{2705}").await.unwrap();
    app.seed_mapping(7, 20, 8, 4, "\u{2705}").await.unwrap();
    app.seed_mapping(9, 99, 1, 5, "\u{2705}").await.unwrap();

    assert_eq!(
        app.admin("/test/rr/list/7").await.unwrap(),
        "20/8/4/\u{2705} 20/9/2/<:a:5> 20/9/3/\u{2705} 30/5/1/\u{1f525}"
    );
    assert_eq!(app.admin("/test/rr/list/11").await.unwrap(), "");
    assert_eq!(
        app.text("/test/rr/list/7", Some(MEMBER)).await.unwrap(),
        shown("forbidden")
    );
    assert_eq!(
        app.text("/test/rr/list/7", None).await.unwrap(),
        shown("unauthenticated")
    );
    assert_eq!(
        app.admin("/test/rr/list/x").await.unwrap(),
        shown("invalid guild id")
    );

    app.guild_directory(7);
    assert_eq!(
        app.admin("/test/rr/page/7").await.unwrap(),
        "4 mappings; channels rules,chat,bots; roles Mod,Member"
    );

    app.discord.reply("GET", "/guilds/7/roles", 500, "{}");
    assert_eq!(
        app.admin("/test/rr/page/7").await.unwrap(),
        "4 mappings; channels rules,chat,bots; roles "
    );
    assert_eq!(
        app.text("/test/rr/page/7", Some(MEMBER)).await.unwrap(),
        shown("forbidden")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn removing_a_mapping_clears_its_reaction_only_when_it_was_this_guilds(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
    app.seed_mapping(7, 20, 8, 4, "\u{2705}").await.unwrap();
    app.seed_mapping(7, 20, 9, 2, "<:a:5>").await.unwrap();
    app.seed_mapping(7, 20, 10, 2, "\u{1f525}").await.unwrap();
    app.seed_mapping(9, 99, 1, 5, "\u{2705}").await.unwrap();
    app.discord.reply(
        "DELETE",
        "/channels/20/messages/8/reactions/%E2%9C%85",
        204,
        "",
    );

    let remove =
        |channel: &'static str, message: &'static str, emoji: &'static str| {
            let app = &app;
            async move {
                app.admin_post("/test/rr/remove", &[
                    ("guild", "7"),
                    ("channel_id", channel),
                    ("message_id", message),
                    ("emoji", emoji),
                ])
                .await
                .unwrap()
            }
        };

    assert_eq!(remove("20", "7", "\u{2705}").await, "ok");
    assert_eq!(remove("99", "1", "\u{2705}").await, "ok");
    assert_eq!(app.discord.count("DELETE", "/channels"), 0);
    assert_eq!(app.mappings(9).await.unwrap(), ["99/1/5/\u{2705}"]);
    assert_eq!(app.mappings(7).await.unwrap().len(), 3);

    assert_eq!(remove("20", "8", "\u{2705}").await, "ok");
    let hits = app.discord.hits("DELETE", "/channels");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "/channels/20/messages/8/reactions/%E2%9C%85");
    assert_eq!(app.mappings(7).await.unwrap().len(), 2);

    assert_eq!(remove("20", "9", "<:a:5>").await, "ok");
    let hits = app.discord.hits("DELETE", "/channels/20/messages/9/");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "/channels/20/messages/9/reactions/a:5");
    assert_eq!(app.mappings(7).await.unwrap(), ["20/10/2/\u{1f525}"]);

    assert_eq!(remove(" 20 ", " 10 ", " \u{1f525} ").await, "ok");
    assert_eq!(app.discord.count("DELETE", "/channels/20/messages/10/"), 1);
    assert_eq!(app.mappings(7).await.unwrap().len(), 0);

    assert_eq!(remove("abc", "8", "\u{2705}").await, shown("invalid channel"));
    assert_eq!(remove("", "8", "\u{2705}").await, shown("invalid channel"));
    assert_eq!(remove("20", "x", "\u{2705}").await, shown("invalid message id"));
    assert_eq!(remove("20", "", "\u{2705}").await, shown("invalid message id"));
    assert_eq!(
        remove("20", "8", "").await,
        shown("Failed to convert emoji to reaction")
    );
    assert_eq!(remove("20", "8", "<:a:0>").await, shown("invalid custom emoji id"));
    assert_eq!(remove("0", "8", "\u{2705}").await, shown("invalid channel"));
    assert_eq!(remove("20", "0", "\u{2705}").await, shown("invalid message id"));
    assert_eq!(
        remove("0", "8", "").await,
        shown("Failed to convert emoji to reaction")
    );
    assert_eq!(
        remove("18446744073709551615", "8", "\u{2705}").await,
        shown("invalid channel")
    );
    assert_eq!(
        remove("20", "18446744073709551615", "\u{2705}").await,
        shown("invalid message id")
    );
    assert_eq!(remove("9223372036854775808", "8", "\u{2705}").await, "ok");

    assert_eq!(
        app.post(
            "/test/rr/remove",
            &[
                ("guild", "7"),
                ("channel_id", "20"),
                ("message_id", "8"),
                ("emoji", "x")
            ],
            Some(MEMBER)
        )
        .await
        .unwrap(),
        shown("forbidden")
    );
    assert_eq!(
        app.post(
            "/test/rr/remove",
            &[
                ("guild", "7"),
                ("channel_id", "20"),
                ("message_id", "8"),
                ("emoji", "x")
            ],
            None
        )
        .await
        .unwrap(),
        shown("unauthenticated")
    );
    assert_eq!(
        app.admin_post("/test/rr/remove", &[("guild", "7"), ("channel_id", "20")])
            .await
            .unwrap(),
        shown("missing field `message_id`")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn adding_a_mapping_validates_before_anything_is_sent_to_discord(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
    app.guild_directory(7);

    let add = |channel: &'static str,
               message: &'static str,
               role: &'static str,
               emoji: &'static str| {
        let app = &app;
        async move {
            app.admin_post("/test/rr/add", &[
                ("guild", "7"),
                ("channel_id", channel),
                ("message_id", message),
                ("role_id", role),
                ("emoji", emoji),
            ])
            .await
            .unwrap()
        }
    };

    assert_eq!(add("", "", "30", "\u{2705}").await, shown("invalid channel"));
    assert_eq!(add("x", "", "30", "\u{2705}").await, shown("invalid channel"));
    assert_eq!(add("20", "", "", "\u{2705}").await, shown("invalid role"));
    assert_eq!(add("20", "", "x", "\u{2705}").await, shown("invalid role"));
    assert_eq!(
        add("20", "", "30", "").await,
        shown("Failed to convert emoji to reaction")
    );
    assert_eq!(
        add("20", "", "30", "<bad>").await,
        shown("Failed to convert emoji to reaction")
    );
    assert_eq!(
        add("20", "", "30", "<:a:0>").await,
        shown("invalid custom emoji id")
    );
    assert_eq!(app.discord.total(), 0);

    assert_eq!(
        add("99", "", "30", "\u{2705}").await,
        shown("that channel is not in this server")
    );
    assert_eq!(
        add("20", "", "99", "\u{2705}").await,
        shown("that role is not in this server")
    );
    assert_eq!(
        add("20", "", "7", "\u{2705}").await,
        shown("that role is not in this server")
    );
    assert_eq!(
        add("0", "", "30", "\u{2705}").await,
        shown("that channel is not in this server")
    );
    assert_eq!(
        add("20", "", "0", "\u{2705}").await,
        shown("that role is not in this server")
    );
    assert_eq!(
        add("20", "abc", "30", "\u{2705}").await,
        shown("invalid message id")
    );
    assert_eq!(add("20", "0", "30", "\u{2705}").await, shown("invalid message id"));

    assert_eq!(app.discord.count("POST", "/"), 0);
    assert_eq!(app.discord.count("PUT", "/"), 0);

    let missing = add("20", "123", "30", "\u{2705}").await;
    assert!(missing.starts_with(&shown("")), "{missing}");
    assert!(missing.contains("404"), "{missing}");
    assert_eq!(app.mappings(7).await.unwrap().len(), 0);

    assert_eq!(
        app.post(
            "/test/rr/add",
            &[
                ("guild", "7"),
                ("channel_id", "20"),
                ("message_id", ""),
                ("role_id", "30"),
                ("emoji", "\u{2705}"),
            ],
            Some(MEMBER)
        )
        .await
        .unwrap(),
        shown("forbidden")
    );
    assert_eq!(app.discord.count("GET", "/guilds/7/channels"), 8);
}

#[sqlx::test(migrations = "../migrations")]
async fn adding_a_mapping_posts_a_panel_or_uses_the_message_and_seeds_the_reaction(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
    app.guild_directory(7);
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
        "PUT",
        "/channels/20/messages/555/reactions/%F0%9F%94%A5/@me",
        403,
        r#"{"message":"Missing Permissions","code":50013}"#,
    );

    let add = |message: &'static str, role: &'static str, emoji: &'static str| {
        let app = &app;
        async move {
            app.admin_post("/test/rr/add", &[
                ("guild", "7"),
                ("channel_id", " 20 "),
                ("message_id", message),
                ("role_id", role),
                ("emoji", emoji),
            ])
            .await
            .unwrap()
        }
    };

    let unseeded = add("", "30", " \u{2705} ").await;
    assert!(unseeded.contains("reaction_roles_guild_id_fkey"), "{unseeded}");
    assert_eq!(app.discord.count("POST", "/channels/20/messages"), 1);
    assert_eq!(app.discord.count("PUT", "/channels"), 0);
    assert_eq!(app.mappings(7).await.unwrap().len(), 0);

    app.seed_guild(7).await.unwrap();
    assert_eq!(add("", "30", " \u{2705} ").await, "ok");
    assert_eq!(app.mappings(7).await.unwrap(), ["20/555/30/\u{2705}"]);
    let posts = app.discord.hits("POST", "/channels/20/messages");
    assert_eq!(posts.len(), 2);
    assert!(posts[1].body.contains("\u{2705} | <@&30>"), "{}", posts[1].body);
    assert!(posts[1].body.contains(r#""type":"rich""#), "{}", posts[1].body);
    assert_eq!(
        app.discord
            .count("PUT", "/channels/20/messages/555/reactions/%E2%9C%85/@me"),
        1
    );

    assert_eq!(
        add("555", "31", "\u{2705}").await,
        shown("that emoji is already mapped on that message")
    );
    assert_eq!(app.discord.count("POST", "/channels/20/messages"), 2);
    assert_eq!(app.mappings(7).await.unwrap().len(), 1);

    assert_eq!(add(" 555 ", "31", "<:a:5>").await, "ok");
    assert_eq!(app.mappings(7).await.unwrap(), [
        "20/555/30/\u{2705}",
        "20/555/31/<:a:5>"
    ]);
    assert_eq!(
        app.discord.count("PUT", "/channels/20/messages/555/reactions/a:5/"),
        1
    );

    let refused = add("555", "30", "\u{1f525}").await;
    assert!(refused.starts_with(&shown("")), "{refused}");
    assert_eq!(app.mappings(7).await.unwrap().len(), 3);

    let listed = app.admin("/test/rr/list/7").await.unwrap();
    assert_eq!(listed, "20/555/31/<:a:5> 20/555/30/\u{2705} 20/555/30/\u{1f525}");
}

fn default_view(
    tier: Plan,
    floors: (i32, i32),
    next: Option<(Plan, (i32, i32))>,
) -> GreetingsView {
    GreetingsView {
        morning_message: String::new(),
        night_message: String::new(),
        morning: Vec::new(),
        night: Vec::new(),
        allowed_channels: Some(Vec::new()),
        channels_locked: false,
        cooldowns: CooldownView {
            user_secs: 15,
            guild_secs: 3,
            floor_user_secs: floors.0,
            floor_guild_secs: floors.1,
            tier,
            next_tier: next.map(|(plan, _)| plan),
            next_floor_user_secs: next.map_or(floors.0, |(_, f)| f.0),
            next_floor_guild_secs: next.map_or(floors.1, |(_, f)| f.1),
        },
    }
}

fn mock_free_guild(app: &Harness, guild: u64) {
    app.discord.get(&format!("/guilds/{guild}"), &guild_json(guild, 1));
    app.discord.get(&format!("/applications/{APP_ID}/commands"), "[]");
    app.discord
        .get(&format!("/applications/{APP_ID}/guilds/{guild}/commands"), "[]");
}

#[sqlx::test(migrations = "../migrations")]
async fn the_greetings_view_starts_at_the_free_plans_defaults_and_saves_show_up(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
    mock_free_guild(&app, 7);
    app.guild_directory(7);

    let mut expected = default_view(Plan::Free, (15, 3), Some((Plan::Pro, (3, 1))));
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&expected));
    assert_eq!(
        app.admin("/test/greetings-page/7").await.unwrap(),
        "3 channels: rules,chat,bots"
    );

    let save = |morning: &str, night: &str| {
        let app = &app;
        let (morning, night) = (morning.to_owned(), night.to_owned());
        async move {
            app.admin_post("/test/greetings/messages", &[
                ("guild", "7"),
                ("morning_message", &morning),
                ("night_message", &night),
            ])
            .await
            .unwrap()
        }
    };

    assert_eq!(save("  Good morning {user}!  ", "Night, {author}").await, "ok");
    expected.morning_message = "Good morning {user}!".to_owned();
    expected.night_message = "Night, {author}".to_owned();
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&expected));

    let too_long = shown(
        "Greeting messages are limited to 1500 characters so the reply still fits once mentions are filled in.",
    );
    assert_eq!(save(&"x".repeat(1501), "kept?").await, too_long);
    assert_eq!(save("fine", &"x".repeat(1501)).await, too_long);
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&expected));
    assert_eq!(save(&"\u{e9}".repeat(1500), "").await, "ok");
    expected.morning_message = "\u{e9}".repeat(1500);
    expected.night_message = String::new();
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&expected));
    assert_eq!(save("", "").await, "ok");
    expected.morning_message = String::new();
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&expected));

    let cooldowns = |user: &'static str, guild: &'static str| {
        let app = &app;
        async move {
            app.admin_post("/test/greetings/cooldowns", &[
                ("guild", "7"),
                ("user_cooldown", user),
                ("guild_cooldown", guild),
            ])
            .await
            .unwrap()
        }
    };
    let invalid = |raw: &str| {
        shown(&format!(
            "`{raw}` isn't a usable cooldown. Enter a whole number of seconds between 0 and 86400."
        ))
    };

    assert_eq!(cooldowns("20", "5").await, "ok");
    expected.cooldowns.user_secs = 20;
    expected.cooldowns.guild_secs = 5;
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&expected));

    assert_eq!(cooldowns("", " ").await, "ok");
    expected.cooldowns.user_secs = 15;
    expected.cooldowns.guild_secs = 3;
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&expected));

    assert_eq!(cooldowns(" 86400 ", "86400").await, "ok");
    expected.cooldowns.user_secs = 86_400;
    expected.cooldowns.guild_secs = 86_400;
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&expected));

    assert_eq!(cooldowns("abc", "3").await, invalid("abc"));
    assert_eq!(cooldowns("15", "-1").await, invalid("-1"));
    assert_eq!(cooldowns("86401", "3").await, invalid("86401"));
    assert_eq!(cooldowns("1.5", "3").await, invalid("1.5"));
    assert_eq!(cooldowns("99999999999", "3").await, invalid("99999999999"));
    assert_eq!(cooldowns("abc", "xyz").await, invalid("abc"));
    assert_eq!(
        cooldowns("14", "3").await,
        shown(
            "On the Free plan the per-member cooldown can't go below 15s. Pro servers can go as low as 3s."
        )
    );
    assert_eq!(
        cooldowns("15", "2").await,
        shown(
            "On the Free plan the server-wide cooldown can't go below 3s. Pro servers can go as low as 1s."
        )
    );
    assert_eq!(
        cooldowns("0", "0").await,
        shown(
            "On the Free plan the per-member cooldown can't go below 15s. Pro servers can go as low as 3s."
        )
    );
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&expected));
}

#[sqlx::test(migrations = "../migrations")]
async fn higher_plans_lower_the_floors_and_stop_the_upgrade_pitch(pool: PgPool) {
    let app = harness(pool).await.unwrap();
    for guild in [11_u64, 12] {
        mock_free_guild(&app, guild);
    }
    app.app
        .entitlements
        .grant(EntitlementScope::Guild(11), Tier::Pro, "test", "pro-11", None)
        .await
        .unwrap();
    app.app
        .entitlements
        .grant(EntitlementScope::Guild(12), Tier::Ultra, "test", "ultra-12", None)
        .await
        .unwrap();

    let pro = default_view(Plan::Pro, (3, 1), None);
    let mut pro_view = pro.clone();
    pro_view.cooldowns.user_secs = 15;
    assert_eq!(
        app.admin("/test/greetings-view/11").await.unwrap(),
        debug(&pro_view)
    );

    let ultra = default_view(Plan::Ultra, (0, 0), None);
    assert_eq!(app.admin("/test/greetings-view/12").await.unwrap(), debug(&ultra));

    let post = |guild: &'static str, user: &'static str, per_guild: &'static str| {
        let app = &app;
        async move {
            app.admin_post("/test/greetings/cooldowns", &[
                ("guild", guild),
                ("user_cooldown", user),
                ("guild_cooldown", per_guild),
            ])
            .await
            .unwrap()
        }
    };

    assert_eq!(
        post("11", "2", "1").await,
        shown(
            "On the Pro plan the per-member cooldown can't go below 3s. That is as low as this command goes."
        )
    );
    assert_eq!(
        post("11", "1", "0").await,
        shown(
            "On the Pro plan the per-member cooldown can't go below 3s. That is as low as this command goes."
        )
    );
    assert_eq!(
        post("11", "3", "0").await,
        shown(
            "On the Pro plan the server-wide cooldown can't go below 1s. That is as low as this command goes."
        )
    );
    assert_eq!(post("11", "3", "1").await, "ok");
    assert_eq!(post("11", "", "").await, "ok");

    assert_eq!(post("12", "0", "0").await, "ok");
    assert_eq!(post("12", "", "").await, "ok");
    let mut saved = ultra.clone();
    saved.cooldowns.user_secs = 0;
    saved.cooldowns.guild_secs = 0;
    assert_eq!(app.admin("/test/greetings-view/12").await.unwrap(), debug(&saved));
}

#[sqlx::test(migrations = "../migrations")]
async fn greeting_images_are_validated_capped_and_removed_by_their_own_guild_only(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
    mock_free_guild(&app, 7);
    app.seed_guild(7).await.unwrap();
    app.seed_guild(11).await.unwrap();

    let add = |kind: &'static str, url: &'static str| {
        let app = &app;
        async move {
            app.admin_post("/test/greetings/add-image", &[
                ("guild", "7"),
                ("kind", kind),
                ("url", url),
            ])
            .await
            .unwrap()
        }
    };
    let remove = |id: &str| {
        let app = &app;
        let id = id.to_owned();
        async move {
            app.admin_post("/test/greetings/remove-image", &[
                ("guild", "7"),
                ("id", &id),
            ])
            .await
            .unwrap()
        }
    };

    assert_eq!(add("morning", "https://example.com/a.gif").await, "ok");
    assert_eq!(add(" night ", "  https://example.com/a.gif  ").await, "ok");
    assert_eq!(add("morning", "https://example.com/b.gif").await, "ok");
    assert_eq!(
        add("morning", "https://example.com/a.gif").await,
        shown("That image link is already in the list.")
    );
    assert_eq!(
        add("morning", "http://example.com/a.gif").await,
        shown(
            "`http://example.com/a.gif` isn't a usable image link. Links must start with `https://`."
        )
    );
    assert_eq!(
        add("morning", "https://").await,
        shown(
            "`https://` isn't a usable image link. Links must start with `https://`."
        )
    );
    assert_eq!(
        add("morning", "https://exa mple.com").await,
        shown(
            "`https://exa mple.com` isn't a usable image link. Links must start with `https://`."
        )
    );
    assert_eq!(
        add("noon", "https://example.com/c.gif").await,
        shown("Unknown greeting type `noon`.")
    );
    assert_eq!(
        add("", "https://example.com/c.gif").await,
        shown("Unknown greeting type ``.")
    );

    assert_eq!(app.images(7, "morning").await.unwrap(), [
        "https://example.com/a.gif",
        "https://example.com/b.gif"
    ]);
    assert_eq!(app.images(7, "night").await.unwrap(), ["https://example.com/a.gif"]);

    let view = app.admin("/test/greetings-view/7").await.unwrap();
    let ids = sqlx::query_scalar!(
        "SELECT id FROM greeting_images WHERE guild_id = 7 ORDER BY id"
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    let expected = {
        let mut view = default_view(Plan::Free, (15, 3), Some((Plan::Pro, (3, 1))));
        view.morning = vec![
            GreetingImageInfo {
                id: ids[0].to_string(),
                url: "https://example.com/a.gif".to_owned(),
            },
            GreetingImageInfo {
                id: ids[2].to_string(),
                url: "https://example.com/b.gif".to_owned(),
            },
        ];
        view.night = vec![GreetingImageInfo {
            id: ids[1].to_string(),
            url: "https://example.com/a.gif".to_owned(),
        }];
        view
    };
    assert_eq!(view, debug(&expected));

    for i in 0..47 {
        sqlx::query!(
            "INSERT INTO greeting_images (guild_id, kind, url) VALUES (7, 'morning', $1)",
            format!("https://example.com/fill-{i}.gif")
        )
        .execute(&app.pool)
        .await
        .unwrap();
    }
    assert_eq!(app.images(7, "morning").await.unwrap().len(), 49);
    assert_eq!(add("morning", "https://example.com/fiftieth.gif").await, "ok");
    assert_eq!(
        add("morning", "https://example.com/fifty-first.gif").await,
        shown(
            "This server already has the maximum of 50 images for that greeting. Remove one before adding another."
        )
    );
    assert_eq!(add("night", "https://example.com/fifty-first.gif").await, "ok");
    assert_eq!(app.images(7, "morning").await.unwrap().len(), 50);

    sqlx::query!(
        "INSERT INTO greeting_images (guild_id, kind, url) VALUES (11, 'morning', 'https://example.com/theirs.gif')"
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let theirs =
        sqlx::query_scalar!("SELECT id FROM greeting_images WHERE guild_id = 11")
            .fetch_one(&app.pool)
            .await
            .unwrap()
            .to_string();

    let gone = shown("that image is not in this server's list");
    assert_eq!(remove(&theirs).await, gone);
    assert_eq!(app.images(11, "morning").await.unwrap().len(), 1);
    assert_eq!(remove("999999").await, gone);
    assert_eq!(remove("abc").await, shown("invalid image id"));
    assert_eq!(remove("").await, shown("invalid image id"));
    assert_eq!(remove("2147483648").await, shown("invalid image id"));

    let first = ids[0].to_string();
    assert_eq!(remove(&first).await, "ok");
    assert_eq!(remove(&first).await, gone);
    assert_eq!(app.images(7, "morning").await.unwrap().len(), 49);
}

#[sqlx::test(migrations = "../migrations")]
async fn members_channel_edits_stop_at_validation_and_the_command_lookup(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
    mock_free_guild(&app, 7);
    app.guild_directory(7);

    let post = |action: &'static str,
                channel: &'static str,
                cookie: Option<&'static str>| {
        let app = &app;
        async move {
            app.post(
                &format!("/test/greetings/{action}"),
                &[("guild", "7"), ("channel_id", channel)],
                cookie,
            )
            .await
            .unwrap()
        }
    };

    for action in ["add-channel", "remove-channel"] {
        assert_eq!(post(action, "", None).await, shown("invalid channel id"));
        assert_eq!(
            post(action, "x", Some(MEMBER)).await,
            shown("invalid channel id")
        );
        assert_eq!(
            post(action, "0", Some(ADMIN)).await,
            shown("invalid channel id")
        );
        assert_eq!(
            post(action, "-5", Some(ADMIN)).await,
            shown("invalid channel id")
        );
        assert_eq!(post(action, "20", None).await, shown("unauthenticated"));
        assert_eq!(post(action, "20", Some(MEMBER)).await, shown("forbidden"));
    }
    assert_eq!(app.discord.total(), 0);

    assert_eq!(
        post("add-channel", "99", Some(ADMIN)).await,
        shown("that channel is not in this server")
    );
    assert_eq!(
        post("add-channel", "18446744073709551615", Some(ADMIN)).await,
        shown("that channel is not in this server")
    );
    assert_eq!(
        post("add-channel", " 20 ", Some(ADMIN)).await,
        shown("/good isn't registered for this server yet")
    );
    assert_eq!(
        post("remove-channel", "20", Some(ADMIN)).await,
        shown("/good isn't registered for this server yet")
    );

    app.discord.reply("GET", "/applications/123456789/guilds/7/commands", 500, "{}");
    let mut unknown = default_view(Plan::Free, (15, 3), Some((Plan::Pro, (3, 1))));
    unknown.allowed_channels = None;
    assert_eq!(app.admin("/test/greetings-view/7").await.unwrap(), debug(&unknown));
}

#[sqlx::test(migrations = "../migrations")]
async fn operators_read_a_guilds_channel_allowlist_but_cannot_change_it(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
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

    let mut expected = default_view(Plan::Free, (15, 3), Some((Plan::Pro, (3, 1))));
    expected.allowed_channels = Some(vec!["21".to_owned(), "22".to_owned()]);
    expected.channels_locked = true;
    assert_eq!(
        app.text("/test/greetings-view/8", Some(OPERATOR)).await.unwrap(),
        debug(&expected)
    );
    assert_eq!(
        app.text("/test/greetings-page/8", Some(OPERATOR)).await.unwrap(),
        "3 channels: rules,chat,bots"
    );
    assert_eq!(
        app.text("/test/greetings-view/8", Some(ADMIN)).await.unwrap(),
        shown("forbidden")
    );

    let post = |action: &'static str, channel: &'static str| {
        let app = &app;
        async move {
            app.post(
                &format!("/test/greetings/{action}"),
                &[("guild", "8"), ("channel_id", channel)],
                Some(OPERATOR),
            )
            .await
            .unwrap()
        }
    };
    let refused = shown(
        "Discord only lets a member with Manage Server change command permissions, so /good can't be changed through operator access.",
    );

    assert_eq!(
        post("add-channel", "99").await,
        shown("that channel is not in this server")
    );
    assert_eq!(
        post("add-channel", "21").await,
        shown("that channel is already on the list")
    );
    assert_eq!(post("add-channel", "20").await, refused);
    assert_eq!(
        post("remove-channel", "20").await,
        shown("that channel is not on this server's list")
    );
    assert_eq!(post("remove-channel", "21").await, refused);
    assert_eq!(app.discord.count("PUT", "/applications"), 0);

    assert_eq!(
        app.post(
            "/test/greetings/messages",
            &[("guild", "8"), ("morning_message", "Hi"), ("night_message", "")],
            Some(OPERATOR)
        )
        .await
        .unwrap(),
        "ok"
    );
    assert_eq!(
        app.post(
            "/test/greetings/add-image",
            &[
                ("guild", "8"),
                ("kind", "night"),
                ("url", "https://example.com/n.gif")
            ],
            Some(OPERATOR)
        )
        .await
        .unwrap(),
        "ok"
    );
    for (guild, listed) in [(9_u64, 90_u64), (10, 89)] {
        let mut entries = vec![(guild - 1, 3_u8, false)];
        entries.extend((1001..1001 + listed).map(|id| (id, 3_u8, true)));
        app.discord.get(&format!("/guilds/{guild}"), &guild_json(guild, 1));
        app.discord
            .get(&format!("/applications/{APP_ID}/guilds/{guild}/commands"), "[]");
        app.discord.get(
            &format!(
                "/applications/{APP_ID}/guilds/{guild}/commands/500/permissions"
            ),
            &permissions_json(guild, 500, &entries),
        );
        app.guild_directory(guild);
    }
    let add_to = |guild: &'static str| {
        let app = &app;
        async move {
            app.post(
                "/test/greetings/add-channel",
                &[("guild", guild), ("channel_id", "20")],
                Some(OPERATOR),
            )
            .await
            .unwrap()
        }
    };
    assert_eq!(
        add_to("9").await,
        shown(
            "Discord allows at most 90 channels per command. Remove one before adding another."
        )
    );
    assert_eq!(add_to("10").await, refused);
    assert_eq!(app.discord.count("PUT", "/applications"), 0);

    expected.morning_message = "Hi".to_owned();
    let rendered = app.text("/test/greetings-view/8", Some(OPERATOR)).await.unwrap();
    assert!(rendered.contains("morning_message: \"Hi\""), "{rendered}");
    assert!(rendered.contains("https://example.com/n.gif"), "{rendered}");
}

#[sqlx::test(migrations = "../migrations")]
async fn every_engagement_call_refuses_the_signed_out_and_non_admins(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let calls: [(&str, Fields); 6] = [
        ("/test/rr/add", vec![
            ("channel_id", "20"),
            ("message_id", ""),
            ("role_id", "30"),
            ("emoji", "x"),
        ]),
        ("/test/rr/remove", vec![
            ("channel_id", "20"),
            ("message_id", "1"),
            ("emoji", "x"),
        ]),
        ("/test/greetings/messages", vec![
            ("morning_message", ""),
            ("night_message", ""),
        ]),
        ("/test/greetings/cooldowns", vec![
            ("user_cooldown", ""),
            ("guild_cooldown", ""),
        ]),
        ("/test/greetings/add-image", vec![
            ("kind", "morning"),
            ("url", "https://a.b"),
        ]),
        ("/test/greetings/remove-image", vec![("id", "1")]),
    ];

    for (path, fields) in calls {
        let mut with_guild = vec![("guild", "7")];
        with_guild.extend(fields.iter().copied());

        assert_eq!(
            app.post(path, &with_guild, None).await.unwrap(),
            shown("unauthenticated"),
            "{path}"
        );
        assert_eq!(
            app.post(path, &with_guild, Some(MEMBER)).await.unwrap(),
            shown("forbidden"),
            "{path}"
        );

        let mut bad_guild = vec![("guild", "abc")];
        bad_guild.extend(fields.iter().copied());
        assert_eq!(
            app.post(path, &bad_guild, Some(ADMIN)).await.unwrap(),
            shown("invalid guild id"),
            "{path}"
        );
    }

    for path in ["/test/greetings-view/7", "/test/rr/list/7", "/test/levels/7"] {
        assert_eq!(
            app.text(path, None).await.unwrap(),
            shown("unauthenticated"),
            "{path}"
        );
        assert_eq!(
            app.text(path, Some(MEMBER)).await.unwrap(),
            shown("forbidden"),
            "{path}"
        );
    }
    assert_eq!(app.discord.total(), 0);
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
