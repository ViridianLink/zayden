//! The Discord sign-in routes, the session cookie they manage, and the
//! request helpers pages use to authorize themselves.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use http_body_util::BodyExt;
use jiff::{SignedDuration, Timestamp};
use sqlx::PgPool;
use topcoat::Result;
use topcoat::context::{Cx, CxTestBuilder};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, HeaderValue, Router, StatusCode, header, route};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use web::auth::{self, GuildAccess, WebRole};
use web::state::{SessionCache, UserGuildsCache, WebState};
use web::util::server_error_text;
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const CLIENT_ID: u64 = 123_456_789;
const REDIRECT_URI: &str = "http://localhost:3000/auth/callback";
/// Seeded with both dashboard roles by the migrations.
const OPERATOR: i64 = 211_486_447_369_322_506;

#[route(GET "/test/members")]
async fn members(cx: &Cx) -> Result<String> {
    Ok(match auth::require_user(cx).await {
        Ok(identity) => identity.user_id.to_string(),
        Err(e) => server_error_text(e.redirect_unauthenticated()?),
    })
}

#[route(GET "/test/whoami")]
async fn whoami(cx: &Cx) -> Result<String> {
    Ok(match auth::current_user_id(cx).await {
        Ok(id) => id.to_string(),
        Err(e) => e.to_string(),
    })
}

#[route(GET "/test/signed-in")]
async fn signed_in(cx: &Cx) -> Result<String> {
    Ok(auth::check_session(cx).await?.to_string())
}

#[route(GET "/test/profile")]
async fn profile(cx: &Cx) -> Result<String> {
    Ok(auth::current_session_user(cx)
        .await?
        .map_or_else(|| "signed out".to_owned(), |user| user.name))
}

#[route(GET "/test/admin")]
async fn admin_only(cx: &Cx) -> Result<String> {
    Ok(match auth::require_role(cx, WebRole::Admin).await {
        Ok(id) => id.to_string(),
        Err(e) => e.to_string(),
    })
}

#[route(GET "/test/guild-admin")]
async fn guild_admin(cx: &Cx) -> Result<String> {
    Ok(match auth::guild_admin_context(cx, "7").await {
        Ok(ctx) if ctx.access == GuildAccess::Member => {
            format!("{} member", ctx.guild_id)
        },
        Ok(ctx) => format!("{} operator", ctx.guild_id),
        Err(e) => e.to_string(),
    })
}

#[route(GET "/test/start-session")]
async fn start(cx: &Cx) -> Result<&'static str> {
    auth::start_session(cx, auth::db_pool(cx)?, 41, "issued-access-token").await?;
    Ok("started")
}

#[route(GET "/test/protected")]
async fn protected() -> Result<&'static str> {
    Ok("inside")
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
        zayden_id: CLIENT_ID,
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
        redirect_uri: REDIRECT_URI.to_owned(),
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

struct Harness {
    router: Router,
    pool: PgPool,
    sessions: SessionCache,
    user_guilds: UserGuildsCache,
}

fn web_state(pool: &PgPool) -> TestResult<WebState> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let config = config();
    let app = Arc::new(ZaydenAppState::new(pool.clone(), &config));
    Ok(WebState::new(app, &config)?)
}

/// The app router, with test-only routes added to its base, so every test
/// also exercises the cookie layer and sign-in routes it installs.
fn harness(pool: PgPool) -> TestResult<Harness> {
    let state = web_state(&pool)?;
    let sessions = state.sessions.clone();
    let user_guilds = state.discord.user_guilds.clone();

    let base = Router::builder()
        .app_context(state)
        .route(members)
        .route(whoami)
        .route(signed_in)
        .route(profile)
        .route(admin_only)
        .route(guild_admin)
        .route(start)
        .route(protected)
        .layer(auth::require_auth("/test/protected"));
    let router = web::router(base);

    Ok(Harness { router, pool, sessions, user_guilds })
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
}

async fn body_text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn location(response: &Response) -> Option<&str> {
    response.headers().get(header::LOCATION)?.to_str().ok()
}

fn set_cookies(response: &Response) -> Vec<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_owned))
        .collect()
}

async fn insert_session(
    pool: &PgPool,
    token: &str,
    user_id: i64,
    ttl: SignedDuration,
) -> TestResult<()> {
    let expires_at = jiff_sqlx::Timestamp::from(Timestamp::now().checked_add(ttl)?);

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

async fn seed(pool: &PgPool) -> TestResult<()> {
    insert_session(pool, "live-token", 41, SignedDuration::from_hours(168)).await?;
    insert_session(pool, "expired-token", 42, SignedDuration::from_hours(-1))
        .await?;
    insert_session(
        pool,
        "operator-token",
        OPERATOR,
        SignedDuration::from_hours(168),
    )
    .await?;
    insert_session(pool, "plain-token", 43, SignedDuration::from_hours(168)).await?;
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn sign_in_sends_the_browser_to_discord_with_a_state_cookie(pool: PgPool) {
    let app = harness(pool).unwrap();

    let response = app.get("/auth/discord", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let cookies = set_cookies(&response);
    assert_eq!(cookies.len(), 1);
    let (state, attributes) = cookies[0]
        .strip_prefix("oauth_state=")
        .and_then(|rest| rest.split_once(';'))
        .unwrap();
    assert_ne!(state, "");
    assert_eq!(attributes, " HttpOnly; SameSite=Lax; Path=/; Max-Age=600");

    assert_eq!(
        location(&response).unwrap(),
        format!(
            "https://discord.com/oauth2/authorize?response_type=code&client_id={CLIENT_ID}\
             &state={state}&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fauth%2Fcallback\
             &scope=identify+guilds+email+applications.commands.permissions.update"
        )
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_callback_without_code_or_state_is_a_bad_request(pool: PgPool) {
    let app = harness(pool).unwrap();

    for path in ["/auth/callback", "/auth/callback?code=c", "/auth/callback?state=s"]
    {
        let response = app.get(path, Some("oauth_state=s")).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(set_cookies(&response), Vec::<String>::new(), "{path}");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn a_callback_with_a_mismatched_state_fails_and_spends_the_cookie(
    pool: PgPool,
) {
    let app = harness(pool).unwrap();

    let response = app
        .get("/auth/callback?code=c&state=returned", Some("oauth_state=remembered"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login?error=auth_failed"));
    let cookies = set_cookies(&response);
    assert_eq!(cookies.len(), 1);
    assert!(
        cookies[0].starts_with("oauth_state=; Path=/; Max-Age=0"),
        "{cookies:?}"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_callback_without_a_state_cookie_fails(pool: PgPool) {
    let app = harness(pool).unwrap();

    let response =
        app.get("/auth/callback?code=c&state=returned", None).await.unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login?error=auth_failed"));
    assert_eq!(set_cookies(&response), Vec::<String>::new());
}

#[sqlx::test(migrations = "../migrations")]
async fn an_empty_state_never_matches(pool: PgPool) {
    let app = harness(pool).unwrap();

    let response =
        app.get("/auth/callback?code=c&state=", Some("oauth_state=")).await.unwrap();

    assert_eq!(location(&response), Some("/login?error=auth_failed"));
}

/// The cookie and the `web_sessions` row share one TTL, so the browser never
/// drops a session the server still honours, or the reverse.
#[sqlx::test(migrations = "../migrations")]
async fn a_started_session_sets_a_seven_day_cookie_and_row(pool: PgPool) {
    let app = harness(pool).unwrap();

    let response = app.get("/test/start-session", None).await.unwrap();
    let cookies = set_cookies(&response);
    assert_eq!(cookies.len(), 1);
    let (token, attributes) = cookies[0]
        .strip_prefix("session=")
        .and_then(|rest| rest.split_once(';'))
        .unwrap();
    assert_eq!(attributes, " HttpOnly; SameSite=Lax; Path=/; Max-Age=604800");
    assert_eq!(token.len(), 64);
    assert!(token.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));

    let identity =
        auth::lookup_session(None, &app.pool, token).await.unwrap().unwrap();
    assert_eq!(identity.user_id, 41);
    assert_eq!(identity.access_token, "issued-access-token");

    let remaining = sqlx::query_scalar!(
        "SELECT EXTRACT(EPOCH FROM expires_at - now())::bigint AS \"remaining!\" \
         FROM web_sessions WHERE token = $1",
        token,
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!((604_800 - 60..=604_800).contains(&remaining), "{remaining}");
}

#[sqlx::test(migrations = "../migrations")]
async fn logout_without_a_session_still_clears_the_cookie(pool: PgPool) {
    let app = harness(pool).unwrap();

    let response = app.get("/logout", None).await.unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));
    assert_eq!(set_cookies(&response), [
        "session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
    ]);
}

#[sqlx::test(migrations = "../migrations")]
async fn logout_deletes_the_row_and_forgets_the_cached_identity(pool: PgPool) {
    let app = harness(pool).unwrap();
    seed(&app.pool).await.unwrap();
    assert_eq!(
        app.text("/test/whoami", Some("session=live-token")).await.unwrap(),
        "41"
    );
    assert!(app.sessions.get("live-token").await.is_some());

    let response = app.get("/logout", Some("session=live-token")).await.unwrap();

    assert_eq!(location(&response), Some("/login"));
    assert_eq!(set_cookies(&response), [
        "session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
    ]);
    assert!(app.sessions.get("live-token").await.is_none());
    assert!(
        auth::lookup_session(None, &app.pool, "live-token").await.unwrap().is_none()
    );
    assert_eq!(
        app.text("/test/whoami", Some("session=live-token")).await.unwrap(),
        "unauthenticated"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn members_only_pages_send_signed_out_visitors_to_login(pool: PgPool) {
    let app = harness(pool).unwrap();
    seed(&app.pool).await.unwrap();

    for cookie in
        [None, Some("session=no-such-token"), Some("session=expired-token")]
    {
        let response = app.get("/test/members", cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{cookie:?}");
        assert_eq!(location(&response), Some("/login"), "{cookie:?}");
    }

    let response =
        app.get("/test/members", Some("session=live-token")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await.unwrap(), "41");
}

#[sqlx::test(migrations = "../migrations")]
async fn the_session_helpers_read_the_cookie(pool: PgPool) {
    let app = harness(pool).unwrap();
    seed(&app.pool).await.unwrap();

    assert_eq!(app.text("/test/whoami", None).await.unwrap(), "unauthenticated");
    assert_eq!(app.text("/test/signed-in", None).await.unwrap(), "false");
    assert_eq!(app.text("/test/profile", None).await.unwrap(), "signed out");
    assert_eq!(
        app.text("/test/whoami", Some("session=live-token")).await.unwrap(),
        "41"
    );
    assert_eq!(
        app.text("/test/signed-in", Some("session=live-token")).await.unwrap(),
        "true"
    );
    assert_eq!(
        app.text("/test/signed-in", Some("session=expired-token")).await.unwrap(),
        "false"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn roles_are_required_from_the_signed_in_user(pool: PgPool) {
    let app = harness(pool).unwrap();
    seed(&app.pool).await.unwrap();

    assert_eq!(app.text("/test/admin", None).await.unwrap(), "unauthenticated");
    assert_eq!(
        app.text("/test/admin", Some("session=live-token")).await.unwrap(),
        "forbidden"
    );
    assert_eq!(
        app.text("/test/admin", Some("session=operator-token")).await.unwrap(),
        OPERATOR.to_string()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn guild_access_comes_from_the_cached_guild_list(pool: PgPool) {
    let app = harness(pool).unwrap();
    seed(&app.pool).await.unwrap();
    app.user_guilds
        .insert(
            41,
            Arc::from([CurrentUserGuild {
                id: Id::new(7),
                name: "Guild 7".to_owned(),
                icon: None,
                owner: false,
                permissions: Permissions::ADMINISTRATOR,
                features: Vec::new(),
            }]),
        )
        .await;
    app.user_guilds
        .insert(
            43,
            Arc::from([CurrentUserGuild {
                id: Id::new(7),
                name: "Guild 7".to_owned(),
                icon: None,
                owner: false,
                permissions: Permissions::SEND_MESSAGES,
                features: Vec::new(),
            }]),
        )
        .await;

    assert_eq!(
        app.text("/test/guild-admin", None).await.unwrap(),
        "unauthenticated"
    );
    assert_eq!(
        app.text("/test/guild-admin", Some("session=no-such-token")).await.unwrap(),
        "unauthenticated"
    );
    assert_eq!(
        app.text("/test/guild-admin", Some("session=live-token")).await.unwrap(),
        "7 member"
    );
    assert_eq!(
        app.text("/test/guild-admin", Some("session=plain-token")).await.unwrap(),
        "forbidden"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn protected_routes_refuse_requests_without_a_live_session(pool: PgPool) {
    let app = harness(pool).unwrap();
    seed(&app.pool).await.unwrap();

    for cookie in
        [None, Some("session=no-such-token"), Some("session=expired-token")]
    {
        let response = app.get("/test/protected", cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{cookie:?}");
        assert_eq!(body_text(response).await.unwrap(), "", "{cookie:?}");
    }

    let response =
        app.get("/test/protected", Some("session=live-token")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await.unwrap(), "inside");
}

/// Nothing but the app's own router and state: removing the cookie layer or
/// the sign-in routes from it fails here.
#[sqlx::test(migrations = "../migrations")]
async fn the_app_router_serves_the_sign_in_routes(pool: PgPool) {
    let app = web::router(Router::builder().app_context(web_state(&pool).unwrap()));
    let get = |path: &str, cookie: Option<&str>| {
        let mut request = Request::get(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        app.handle(request.body(Body::empty()).unwrap())
    };

    let response = get("/logout", Some("session=unknown")).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));
    assert_eq!(set_cookies(&response), [
        "session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
    ]);

    let response = get("/auth/discord", None).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(set_cookies(&response)[0].starts_with("oauth_state="));

    let response = get("/auth/callback?code=c&state=s", Some("oauth_state=t")).await;
    assert_eq!(location(&response), Some("/login?error=auth_failed"));
}

/// Without the cookie layer the helpers report it instead of panicking.
#[sqlx::test(migrations = "../migrations")]
async fn helpers_without_a_cookie_layer_fail_cleanly(pool: PgPool) {
    let cx = CxTestBuilder::new().app_context(web_state(&pool).unwrap()).build();

    let error = auth::current_session_identity(&cx).await.err().unwrap();
    assert_eq!(error.to_string(), "missing cookie jar");

    let error = auth::current_session_identity(&CxTestBuilder::new().build())
        .await
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "missing database pool");
}

/// Topcoat drops a `Cookie` header holding any non-ASCII byte, so a foreign
/// non-ASCII cookie on the host hides the session cookie sent beside it.
#[sqlx::test(migrations = "../migrations")]
async fn a_non_ascii_cookie_header_reads_as_signed_out(pool: PgPool) {
    let app = harness(pool).unwrap();
    seed(&app.pool).await.unwrap();
    let header_value =
        HeaderValue::from_bytes("session=live-token; x=\u{e9}".as_bytes()).unwrap();

    let request = Request::get("/test/whoami")
        .header(header::COOKIE, header_value)
        .body(Body::empty())
        .unwrap();
    let response = app.router.handle(request).await;

    assert_eq!(body_text(response).await.unwrap(), "unauthenticated");
    assert_eq!(
        app.text("/test/whoami", Some("session=live-token; x=e")).await.unwrap(),
        "41"
    );
}

/// A members-only page renders a session-store failure inline, with the
/// dashboard's error prefix, rather than failing the whole response.
#[sqlx::test(migrations = "../migrations")]
async fn a_session_store_failure_renders_inline_on_members_pages(pool: PgPool) {
    let app = harness(pool).unwrap();
    app.pool.close().await;

    let response =
        app.get("/test/members", Some("session=no-such-token")).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let text = body_text(response).await.unwrap();
    assert!(text.starts_with("error running server function: "), "{text}");
    assert_ne!(text, "error running server function: unauthenticated");
}

/// A protected route answers a session-store failure with an empty 500.
#[sqlx::test(migrations = "../migrations")]
async fn protected_routes_fail_closed_when_the_session_store_is_down(pool: PgPool) {
    let app = harness(pool).unwrap();
    app.pool.close().await;

    let response =
        app.get("/test/protected", Some("session=no-such-token")).await.unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body_text(response).await.unwrap(), "");
}
