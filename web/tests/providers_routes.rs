//! The Ko-fi, Patreon and YouTube webhooks and the Patreon and YouTube
//! connect flows, driven through the router in-process.
//!
//! Each `#[sqlx::test]` runs a group of scenarios with distinct ids against one
//! database. The harness takes every test pool's connections from one shared
//! budget of 20, and a pool that backs a full app state does not give them back
//! until the process ends, so a binary can hold only about 20 such tests; every
//! database creation and drop also waits on a Postgres checkpoint.

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use hmac::{Hmac, KeyInit, Mac};
use http_body_util::BodyExt;
use jiff::{SignedDuration, Timestamp};
use md5::Md5;
use sha1::Sha1;
use sqlx::PgPool;
use sqlx::postgres::{PgListener, PgPoolOptions};
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Method, OriginPolicy, Router, StatusCode, header};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use web::document::STYLESHEET;
use web::providers;
use web::state::WebState;
use web::util::{email_hash, hex_encode};
use zayden_app::config::{BotConfig, PatreonConfig, YoutubeConfig};
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const KOFI_TOKEN: &str = "kofi-secret";
const FORM: &str = "application/x-www-form-urlencoded";
const ADMIN: i64 = 41;
const MEMBER: i64 = 43;
const GUILD: u64 = 7;
const WEBHOOKS: [&str; 3] =
    ["/webhooks/kofi", "/webhooks/patreon", "/webhooks/youtube"];
const CROSS_SITE: [(&str, &str); 3] = [
    ("origin", "https://ko-fi.com"),
    ("sec-fetch-site", "cross-site"),
    ("content-type", FORM),
];

fn config(connectable: bool) -> BotConfig {
    BotConfig {
        discord_token: "test-bot-token".to_owned(),
        bungie_api_key: String::new(),
        ai_provider_key: String::new(),
        google_api_key: "api-key".to_owned(),
        discord_client_secret: "test-client-secret".to_owned(),
        spotify: None,
        ai_api_endpoint: String::new(),
        ai_model: String::new(),
        ai_model_structured: String::new(),
        ai_model_pro: String::new(),
        bot_owner: 0,
        zayden_guild: 0,
        llamad2_guild: 0,
        zayden_id: 123_456_789,
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
        kofi_verification_token: Some(KOFI_TOKEN.to_owned()),
        patreon: connectable.then(|| PatreonConfig {
            client_id: "patreon-client".to_owned(),
            client_secret: "patreon-secret".to_owned(),
            redirect_uri: "http://localhost:3000/patreon/callback".to_owned(),
        }),
        youtube: connectable.then(|| YoutubeConfig {
            client_id: "google-client".to_owned(),
            client_secret: "google-secret".to_owned(),
            redirect_uri: "http://localhost:3000/youtube/callback".to_owned(),
            webhook_uri: "http://localhost:3000/webhooks/youtube".to_owned(),
        }),
        discord_sku_pro: None,
        discord_sku_ultra: None,
        radio_stations: Arc::from(Vec::new()),
    }
}

fn guild(permissions: Permissions) -> Arc<[CurrentUserGuild]> {
    Arc::from([CurrentUserGuild {
        id: Id::new(GUILD),
        name: "Guild 7".to_owned(),
        icon: None,
        owner: false,
        permissions,
        features: Vec::new(),
    }])
}

/// App context over `pool`, with the two test users' guild lists cached so no
/// request reaches Discord.
async fn state(pool: PgPool, connectable: bool) -> TestResult<WebState> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let config = config(connectable);
    let state =
        WebState::new(Arc::new(ZaydenAppState::new(pool, &config)), &config)?;
    let guilds = &state.discord.user_guilds;
    guilds.insert(ADMIN, guild(Permissions::MANAGE_GUILD)).await;
    guilds.insert(MEMBER, guild(Permissions::SEND_MESSAGES)).await;
    Ok(state)
}

struct App {
    router: Router,
}

/// The provider routes on their own, with the given cross-site policy.
async fn providers_app(
    pool: PgPool,
    connectable: bool,
    policy: OriginPolicy,
) -> TestResult<App> {
    let router = providers::routes(
        Router::builder()
            .app_context(state(pool, connectable).await?)
            .origin_policy(policy),
    )
    .cookies()
    .build();
    Ok(App { router })
}

async fn standard(pool: PgPool) -> TestResult<App> {
    providers_app(pool, true, providers::origin_policy()).await
}

async fn unconfigured(pool: PgPool) -> TestResult<App> {
    providers_app(pool, false, providers::origin_policy()).await
}

/// The application router, built the way the server builds it. Its database
/// is never reached: only requests that finish before any query are sent.
async fn application() -> TestResult<App> {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy("postgres://127.0.0.1:1/unused")?;
    let bundle = AssetBundle::load_dir(bundle_dir()?)?;
    let router = web::router(
        Router::builder().assets(bundle).app_context(state(pool, true).await?),
    );
    Ok(App { router })
}

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-providers-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("tailwind-0123456789abcdef.css"), "")?;
    std::fs::write(dir.join("topcoat-0123456789abcdef.js"), "")?;
    std::fs::write(
        dir.join("manifest.toml"),
        format!(
            "version = 1\n\n\
             [[assets]]\nid = {}\nfile = \"tailwind-0123456789abcdef.css\"\nhash = \"0\"\ncontent_type = \"text/css\"\n\n\
             [[assets]]\nid = {}\nfile = \"topcoat-0123456789abcdef.js\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n",
            STYLESHEET.id().as_u64(),
            topcoat::runtime::SCRIPT.id().as_u64(),
        ),
    )?;
    Ok(dir)
}

impl App {
    async fn send(
        &self,
        method: Method,
        path: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> TestResult<Response> {
        let mut request = Request::builder().method(method).uri(path);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        Ok(self.router.handle(request.body(Body::from(body))?).await)
    }

    async fn get(&self, path: &str, cookie: Option<&str>) -> TestResult<Response> {
        let headers: Vec<(&str, &str)> =
            cookie.map(|c| ("cookie", c)).into_iter().collect();
        self.send(Method::GET, path, &headers, Vec::new()).await
    }

    async fn post(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> TestResult<Response> {
        self.send(Method::POST, path, headers, body.as_bytes().to_vec()).await
    }

    async fn post_form(&self, path: &str, body: &str) -> TestResult<Response> {
        self.post(path, &[("content-type", FORM)], body).await
    }

    async fn kofi(&self, payload: &str) -> TestResult<Response> {
        self.post_form("/webhooks/kofi", &format!("data={}", encode(payload))).await
    }
}

async fn session(pool: &PgPool, token: &str, user_id: i64) -> TestResult<()> {
    let expires_at = jiff_sqlx::Timestamp::from(
        Timestamp::now().checked_add(SignedDuration::from_hours(24))?,
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

async fn sign_in(pool: &PgPool) -> TestResult<()> {
    session(pool, "admin-token", ADMIN).await?;
    session(pool, "member-token", MEMBER).await
}

fn encode(raw: &str) -> String {
    raw.bytes().fold(String::new(), |mut out, byte| {
        if byte.is_ascii_alphanumeric() {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
        out
    })
}

async fn text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn header_text(response: &Response, name: header::HeaderName) -> Option<&str> {
    response.headers().get(name)?.to_str().ok()
}

fn location(response: &Response) -> Option<&str> {
    header_text(response, header::LOCATION)
}

fn set_cookies(response: &Response) -> Vec<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_owned))
        .collect()
}

async fn assert_acknowledged(response: Response, what: &str) -> TestResult<()> {
    assert_eq!(response.status(), StatusCode::OK, "{what}");
    assert!(response.headers().get(header::CONTENT_TYPE).is_none(), "{what}");
    assert_eq!(text(response).await?, "", "{what}");
    Ok(())
}

fn kofi_payload(
    token: &str,
    email: &str,
    transaction: &str,
    paying: bool,
) -> String {
    format!(
        r#"{{"verification_token":"{token}","kofi_transaction_id":"{transaction}","email":"{email}","type":"Subscription","is_subscription_payment":{paying},"is_first_subscription_payment":false,"timestamp":"2026-10-01T00:00:00Z","message_id":null}}"#
    )
}

async fn link_email(pool: &PgPool, email: &str, user_id: i64) -> TestResult<()> {
    sqlx::query!(
        "INSERT INTO kofi_links (email_hash, discord_user_id) VALUES ($1, $2)",
        email_hash(email),
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn kofi_grants_to(pool: &PgPool, user_id: i64) -> TestResult<i64> {
    Ok(sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM entitlements
           WHERE provider = 'kofi' AND scope_type = 'user' AND scope_id = $1"#,
        user_id
    )
    .fetch_one(pool)
    .await?)
}

#[sqlx::test(migrations = "../migrations")]
async fn kofi_refuses_what_is_not_a_single_data_form(pool: PgPool) {
    let app = standard(pool).await.unwrap();

    let response = app
        .post("/webhooks/kofi", &[("content-type", "application/json")], "{}")
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(
        text(response).await.unwrap(),
        "Form requests must have `Content-Type: application/x-www-form-urlencoded`"
    );

    let response = app.post("/webhooks/kofi", &[], "data=x").await.unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let response = app.post_form("/webhooks/kofi", "other=1").await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        text(response).await.unwrap(),
        "Failed to deserialize form body: missing field `data`"
    );

    let response = app.post_form("/webhooks/kofi", "data=a&data=b").await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        text(response).await.unwrap(),
        "Failed to deserialize form body: duplicate field `data`"
    );

    for path in ["/webhooks/kofi", "/webhooks/patreon"] {
        let response = app.get(path, None).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{path}");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn kofi_records_subscriptions_for_the_linked_account(pool: PgPool) {
    let app = standard(pool.clone()).await.unwrap();

    link_email(&pool, "unusable@example.com", 98).await.unwrap();
    let donation =
        kofi_payload(KOFI_TOKEN, "unusable@example.com", "unusable-4", true)
            .replace("Subscription", "Donation");
    for (what, payload) in [
        ("not json", "not json".to_owned()),
        (
            "wrong token",
            kofi_payload("wrong", "unusable@example.com", "unusable-1", true),
        ),
        (
            "empty token",
            kofi_payload("", "unusable@example.com", "unusable-2", true),
        ),
        ("donation", donation),
    ] {
        let response = app.kofi(&payload).await.unwrap();
        assert_acknowledged(response, what).await.unwrap();
    }
    assert_eq!(kofi_grants_to(&pool, 98).await.unwrap(), 0);

    let response = app
        .kofi(&kofi_payload(KOFI_TOKEN, "stranger@example.com", "stranger-1", true))
        .await
        .unwrap();
    assert_acknowledged(response, "unlinked email").await.unwrap();
    let strangers = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM entitlements WHERE external_id = 'stranger-1'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(strangers, 0);

    link_email(&pool, "pat@example.com", 99).await.unwrap();
    let response = app
        .kofi(&kofi_payload(KOFI_TOKEN, " Pat@Example.com ", "txn-1", true))
        .await
        .unwrap();
    assert_acknowledged(response, "grant").await.unwrap();

    let row = sqlx::query!(
        r#"SELECT tier, scope_type, scope_id,
                  (expires_at > now() + interval '31 days'
                   AND expires_at < now() + interval '33 days') AS "about_a_month!"
           FROM entitlements WHERE provider = 'kofi' AND external_id = 'txn-1'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.tier, "pro");
    assert_eq!(row.scope_type, "user");
    assert_eq!(row.scope_id, 99);
    assert!(row.about_a_month);

    let response = app
        .kofi(&kofi_payload(KOFI_TOKEN, "pat@example.com", "txn-2", false))
        .await
        .unwrap();
    assert_acknowledged(response, "revoke").await.unwrap();
    assert_eq!(kofi_grants_to(&pool, 99).await.unwrap(), 0);

    link_email(&pool, "hosting@example.com", 97).await.unwrap();
    let hosting = kofi_payload(KOFI_TOKEN, "hosting@example.com", "host-1", true)
        .replace(
            r#""message_id":null"#,
            r#""message_id":null,"tier_name":"£5 Small""#,
        );
    let response = app.kofi(&hosting).await.unwrap();
    assert_acknowledged(response, "hosting tier").await.unwrap();
    assert_eq!(kofi_grants_to(&pool, 97).await.unwrap(), 0);
}

/// A provider server posts without a same-site `Origin`: the webhooks must
/// get through and nothing else may.
async fn assert_origin_policy(app: &App, label: &str) -> TestResult<()> {
    for path in WEBHOOKS {
        let response = app.post(path, &CROSS_SITE, "data=not+json").await?;
        assert_eq!(response.status(), StatusCode::OK, "{label} {path}");
    }

    for path in [
        "/patreon/connect",
        "/patreon/callback",
        "/youtube/connect",
        "/youtube/callback",
        "/webhooks",
        "/webhooks/kofi/extra",
        "/webhooks/youtube/extra",
        "/auth/callback",
        "/logout",
        "/anything-else",
    ] {
        let response = app.post(path, &CROSS_SITE, "").await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{label} {path}");
    }

    let same_site = [("origin", "http://localhost"), ("host", "localhost")];
    let response = app.post("/patreon/connect", &same_site, "").await?;
    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "{label}: only the cross-site post is refused"
    );
    Ok(())
}

#[tokio::test]
async fn the_application_router_exempts_only_the_webhooks_from_the_origin_check() {
    let app = application().await.unwrap();

    assert_origin_policy(&app, "application router").await.unwrap();

    let response = app.get("/patreon/connect?guild=7", None).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(text(response).await.unwrap(), "");
}

#[tokio::test]
async fn the_provider_policy_exempts_only_the_webhooks_from_the_origin_check() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy("postgres://127.0.0.1:1/unused")
        .unwrap();
    let app =
        providers_app(pool.clone(), true, providers::origin_policy()).await.unwrap();

    assert_origin_policy(&app, "providers router").await.unwrap();
    assert_eq!(providers::WEBHOOK_PATHS, WEBHOOKS);

    let strict = providers_app(pool, true, OriginPolicy::new()).await.unwrap();
    for path in WEBHOOKS {
        let response = strict.post(path, &CROSS_SITE, "data=x").await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
    }
}

const CAMPAIGN: &str = "campaign-1";
const WEBHOOK_SECRET: &str = "patreon-webhook-secret";

fn post_body() -> String {
    format!(
        r#"{{"data":{{"id":"post-1","type":"post","attributes":{{"title":"Hello","url":"/posts/post-1","published_at":"2026-10-01T12:00:00+00:00","is_public":true}},"relationships":{{"campaign":{{"data":{{"id":"{CAMPAIGN}","type":"campaign"}}}}}}}}}}"#
    )
}

fn patreon_signature(body: &str, secret: &str) -> TestResult<String> {
    let mut mac = Hmac::<Md5>::new_from_slice(secret.as_bytes())?;
    mac.update(body.as_bytes());
    Ok(hex_encode(&mac.finalize().into_bytes()))
}

async fn seed_patreon(pool: &PgPool) -> TestResult<()> {
    sqlx::query!("INSERT INTO guilds (id) VALUES ($1)", GUILD.cast_signed())
        .execute(pool)
        .await?;
    sqlx::query!(
        "INSERT INTO patreon_campaigns (campaign_id) VALUES ($1)",
        CAMPAIGN
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        "INSERT INTO patreon_oauth \
             (guild_id, campaign_id, access_token, refresh_token, expires_at, \
              webhook_id, webhook_secret, connected_by) \
         VALUES ($1, $2, 'a', 'r', now() + interval '1 day', 'wh', $3, $4)",
        GUILD.cast_signed(),
        CAMPAIGN,
        WEBHOOK_SECRET,
        ADMIN
    )
    .execute(pool)
    .await?;
    announce_patreon(pool).await
}

async fn announce_patreon(pool: &PgPool) -> TestResult<()> {
    sqlx::query!(
        "INSERT INTO patreon_announce (guild_id, channel_id) VALUES ($1, 1)",
        GUILD.cast_signed()
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn patreon_posts(pool: &PgPool) -> TestResult<i64> {
    Ok(sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM patreon_posts"#)
        .fetch_one(pool)
        .await?)
}

async fn patreon_event(
    app: &App,
    event: Option<&str>,
    signature: Option<&str>,
    body: &str,
) -> TestResult<Response> {
    let mut headers = Vec::new();
    if let Some(event) = event {
        headers.push(("x-patreon-event", event));
    }
    if let Some(signature) = signature {
        headers.push(("x-patreon-signature", signature));
    }
    app.post("/webhooks/patreon", &headers, body).await
}

#[sqlx::test(migrations = "../migrations")]
async fn patreon_stores_only_signed_posts_of_an_announced_campaign(pool: PgPool) {
    let app = standard(pool.clone()).await.unwrap();
    seed_patreon(&pool).await.unwrap();
    let mut listener = PgListener::connect_with(&pool).await.unwrap();
    listener.listen("patreon_post").await.unwrap();

    let body = post_body();
    let good = patreon_signature(&body, WEBHOOK_SECRET).unwrap();
    let forged = patreon_signature(&body, "another-secret").unwrap();

    let refused = [
        (None, Some(good.as_str()), body.as_str()),
        (Some("members:create"), Some(good.as_str()), body.as_str()),
        (Some("posts:publish"), Some(good.as_str()), "not json"),
        (Some("posts:publish"), None, body.as_str()),
        (Some("posts:publish"), Some(""), body.as_str()),
        (Some("posts:publish"), Some(forged.as_str()), body.as_str()),
        (Some("posts:publish"), Some("zz"), body.as_str()),
    ];
    for (event, signature, payload) in refused {
        let response = patreon_event(&app, event, signature, payload).await.unwrap();
        assert_acknowledged(response, &format!("{event:?} {signature:?}"))
            .await
            .unwrap();
    }
    assert_eq!(patreon_posts(&pool).await.unwrap(), 0);

    sqlx::query!("DELETE FROM patreon_announce").execute(&pool).await.unwrap();
    let response = patreon_event(&app, Some("posts:publish"), Some(&good), &body)
        .await
        .unwrap();
    assert_acknowledged(response, "campaign nobody announces").await.unwrap();
    assert_eq!(patreon_posts(&pool).await.unwrap(), 0);
    announce_patreon(&pool).await.unwrap();

    for attempt in ["first", "replay"] {
        let response =
            patreon_event(&app, Some("posts:publish"), Some(&good), &body)
                .await
                .unwrap();
        assert_acknowledged(response, attempt).await.unwrap();
    }
    assert_eq!(patreon_posts(&pool).await.unwrap(), 1);

    let notification = tokio::time::timeout(Duration::from_secs(5), listener.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(notification.payload(), "post-1");

    let second =
        tokio::time::timeout(Duration::from_millis(300), listener.recv()).await;
    assert!(second.is_err(), "a replayed post must not notify again");
}

fn channel_id(n: u32) -> String {
    format!("UC{n:022}")
}

fn topic(channel: &str) -> String {
    format!("https://www.youtube.com/xml/feeds/videos.xml?channel_id={channel}")
}

async fn seed_channel(pool: &PgPool, channel: &str) -> TestResult<()> {
    sqlx::query!(
        "INSERT INTO youtube_channels (channel_id, title, uploads_playlist_id, websub_secret) \
         VALUES ($1, 'Channel', 'UUabc', 'hub-secret')",
        channel
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Connects `channel` to guild `guild_id`, optionally with an announce
/// channel so uploads are forwarded.
async fn connect_channel(
    pool: &PgPool,
    channel: &str,
    guild_id: i64,
    announced: bool,
) -> TestResult<()> {
    sqlx::query!("INSERT INTO guilds (id) VALUES ($1)", guild_id)
        .execute(pool)
        .await?;
    sqlx::query!(
        "INSERT INTO youtube_connections (guild_id, channel_id, connected_by) \
         VALUES ($1, $2, $3)",
        guild_id,
        channel,
        ADMIN
    )
    .execute(pool)
    .await?;

    if announced {
        sqlx::query!(
            "INSERT INTO youtube_announce (guild_id, channel_id) VALUES ($1, 1)",
            guild_id
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn has_lease(pool: &PgPool, channel: &str) -> TestResult<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT websub_expires_at IS NOT NULL AS "leased!"
           FROM youtube_channels WHERE channel_id = $1"#,
        channel
    )
    .fetch_one(pool)
    .await?)
}

fn verify_path(channel: &str, mode: &str, extra: &str) -> String {
    format!(
        "/webhooks/youtube?channel={channel}&hub.mode={mode}&hub.topic={}&hub.challenge=abc123{extra}",
        encode(&topic(channel))
    )
}

#[sqlx::test(migrations = "../migrations")]
async fn youtube_verification_agrees_only_with_what_is_wanted(pool: PgPool) {
    let app = standard(pool.clone()).await.unwrap();
    let wanted = channel_id(1);
    let unwanted = channel_id(2);
    seed_channel(&pool, &wanted).await.unwrap();
    seed_channel(&pool, &unwanted).await.unwrap();
    connect_channel(&pool, &wanted, 11, false).await.unwrap();

    let full = verify_path(&wanted, "subscribe", "");
    for dropped in ["channel", "hub.mode", "hub.topic", "hub.challenge"] {
        let kept: Vec<&str> = full
            .split_once('?')
            .map_or_default(|(_, query)| query)
            .split('&')
            .filter(|pair| !pair.starts_with(&format!("{dropped}=")))
            .collect();
        let response = app
            .get(&format!("/webhooks/youtube?{}", kept.join("&")), None)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{dropped}");
        assert_eq!(text(response).await.unwrap(), "", "{dropped}");
    }

    let other = full.replace(&wanted, &channel_id(3));
    let response = app.get(&other, None).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "topic mismatch");

    for (what, path) in [
        ("unsubscribe while wanted", verify_path(&wanted, "unsubscribe", "")),
        ("unknown mode", verify_path(&wanted, "renew", "")),
        ("subscribe while unwanted", verify_path(&unwanted, "subscribe", "")),
    ] {
        let response = app.get(&path, None).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{what}");
    }

    let response = app
        .get(&verify_path(&wanted, "subscribe", "&hub.lease_seconds=soon"), None)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        text(response).await.unwrap(),
        "Failed to deserialize query string: hub.lease_seconds: invalid digit found in string"
    );

    let response = app
        .get(&verify_path(&wanted, "subscribe", "&hub.mode=unsubscribe"), None)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        text(response).await.unwrap(),
        "Failed to deserialize query string: duplicate field `hub.mode`"
    );
    assert!(!has_lease(&pool, &wanted).await.unwrap(), "refusals set no lease");

    let response = app
        .get(&verify_path(&wanted, "subscribe", "&hub.lease_seconds=1000"), None)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_text(&response, header::CONTENT_TYPE),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(text(response).await.unwrap(), "abc123");
    assert!(has_lease(&pool, &wanted).await.unwrap());

    let empty = verify_path(&wanted, "subscribe", "")
        .replace("hub.challenge=abc123", "hub.challenge=");
    let response = app.get(&empty, None).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await.unwrap(), "");

    sqlx::query!(
        "UPDATE youtube_channels SET websub_expires_at = now() + interval '1 day' \
         WHERE channel_id = $1",
        unwanted
    )
    .execute(&pool)
    .await
    .unwrap();
    let response =
        app.get(&verify_path(&unwanted, "unsubscribe", ""), None).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await.unwrap(), "abc123");
    assert!(!has_lease(&pool, &unwanted).await.unwrap());
}

fn youtube_signature(body: &str, secret: &str) -> TestResult<String> {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret.as_bytes())?;
    mac.update(body.as_bytes());
    Ok(format!("sha1={}", hex_encode(&mac.finalize().into_bytes())))
}

async fn youtube_notification(
    app: &App,
    query: &str,
    signature: Option<&str>,
    body: &str,
) -> TestResult<Response> {
    let headers: Vec<(&str, &str)> =
        signature.map(|s| ("x-hub-signature", s)).into_iter().collect();
    app.post(&format!("/webhooks/youtube{query}"), &headers, body).await
}

async fn expect_silence(listener: &mut PgListener, what: &str) {
    let heard =
        tokio::time::timeout(Duration::from_millis(300), listener.recv()).await;
    assert!(heard.is_err(), "{what}");
}

#[sqlx::test(migrations = "../migrations")]
async fn youtube_forwards_only_signed_notifications_of_announced_channels(
    pool: PgPool,
) {
    let app = standard(pool.clone()).await.unwrap();
    let announced = channel_id(1);
    let quiet = channel_id(2);
    seed_channel(&pool, &announced).await.unwrap();
    seed_channel(&pool, &quiet).await.unwrap();
    connect_channel(&pool, &announced, 12, true).await.unwrap();
    connect_channel(&pool, &quiet, 13, false).await.unwrap();
    let mut listener = PgListener::connect_with(&pool).await.unwrap();
    listener.listen("youtube_upload").await.unwrap();

    let body = "<feed/>";
    let query = format!("?channel={announced}");
    let forged = youtube_signature(body, "another-secret").unwrap();
    let signature = youtube_signature(body, "hub-secret").unwrap();

    for (what, response) in [
        ("no channel", youtube_notification(&app, "", None, body).await),
        (
            "unknown channel",
            youtube_notification(&app, "?channel=unknown", None, body).await,
        ),
        ("no signature", youtube_notification(&app, &query, None, body).await),
        (
            "forged signature",
            youtube_notification(&app, &query, Some(&forged), body).await,
        ),
        (
            "non-hex signature",
            youtube_notification(&app, &query, Some("sha1=zz"), body).await,
        ),
        (
            "unannounced channel",
            youtube_notification(
                &app,
                &format!("?channel={quiet}"),
                Some(&signature),
                body,
            )
            .await,
        ),
    ] {
        assert_acknowledged(response.unwrap(), what).await.unwrap();
        expect_silence(&mut listener, what).await;
    }

    let response =
        youtube_notification(&app, &query, Some(&signature), body).await.unwrap();
    assert_acknowledged(response, "signed notification").await.unwrap();

    let notification = tokio::time::timeout(Duration::from_secs(5), listener.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(notification.payload(), announced);
}

const FLOWS: [(&str, &str); 2] = [("patreon", "patreon"), ("youtube", "youtube")];

const PATREON_AUTHORIZE: &str = "https://www.patreon.com/oauth2/authorize?response_type=code&client_id=patreon-client&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fpatreon%2Fcallback&scope=identity+campaigns+campaigns.posts+w%3Acampaigns.webhook&state=";
const GOOGLE_AUTHORIZE: &str = "https://accounts.google.com/o/oauth2/v2/auth?response_type=code&client_id=google-client&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fyoutube%2Fcallback&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fyoutube.readonly&access_type=online&prompt=select_account&state=";

fn outcome_location(flow: &str, guild: &str, outcome: &str) -> String {
    format!("/guild/{guild}/settings/{flow}?{flow}={outcome}")
}

#[sqlx::test(migrations = "../migrations")]
async fn the_connect_routes_start_a_flow_only_for_a_guild_admin(pool: PgPool) {
    let app = standard(pool.clone()).await.unwrap();
    let bare = unconfigured(pool.clone()).await.unwrap();
    sign_in(&pool).await.unwrap();

    for (flow, _) in FLOWS {
        for step in ["connect?guild=7", "callback?code=c&state=n.7"] {
            let path = format!("/{flow}/{step}");
            for cookie in [None, Some("session=no-such-token")] {
                let response = app.get(&path, cookie).await.unwrap();
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
                assert_eq!(set_cookies(&response), Vec::<String>::new());
                assert_eq!(text(response).await.unwrap(), "", "{path}");
            }
        }

        let connect = format!("/{flow}/connect");
        let response = app.get(&connect, Some("session=admin-token")).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{flow}");
        assert_eq!(
            text(response).await.unwrap(),
            "Failed to deserialize query string: missing field `guild`"
        );

        let response = bare
            .get(&format!("{connect}?guild=7"), Some("session=admin-token"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{flow}");
        assert_eq!(
            location(&response),
            Some(outcome_location(flow, "7", "unconfigured").as_str())
        );
        assert_eq!(set_cookies(&response), Vec::<String>::new());

        for (cookie, guild) in [
            ("session=member-token", "7"),
            ("session=admin-token", "8"),
            ("session=admin-token", "not-a-guild"),
        ] {
            let response = app
                .get(&format!("{connect}?guild={guild}"), Some(cookie))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SEE_OTHER, "{cookie} {guild}");
            assert_eq!(
                location(&response),
                Some(outcome_location(flow, guild, "forbidden").as_str()),
                "{cookie} {guild}"
            );
            assert_eq!(set_cookies(&response), Vec::<String>::new());
        }

        let wrong_method = app
            .send(Method::POST, &format!("{connect}?guild=7"), &[], Vec::new())
            .await
            .unwrap();
        assert_eq!(wrong_method.status(), StatusCode::METHOD_NOT_ALLOWED, "{flow}");
        let allow = header_text(&wrong_method, header::ALLOW).unwrap_or_default();
        assert!(allow.contains("GET"), "{allow}");
        assert_eq!(text(wrong_method).await.unwrap(), "", "{flow}");
    }

    let mut nonces = Vec::new();
    for (flow, authorize) in
        [("patreon", PATREON_AUTHORIZE), ("youtube", GOOGLE_AUTHORIZE)]
    {
        for _ in 0..2 {
            let response = app
                .get(
                    &format!("/{flow}/connect?guild=7"),
                    Some("session=admin-token"),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SEE_OTHER);

            let cookies = set_cookies(&response);
            assert_eq!(cookies.len(), 1);
            let (nonce, attributes) = cookies[0]
                .strip_prefix(&format!("{flow}_oauth_state="))
                .and_then(|rest| rest.split_once(';'))
                .unwrap();
            assert_eq!(nonce.len(), 64);
            assert!(nonce.bytes().all(|b| b.is_ascii_hexdigit()));
            assert_eq!(attributes, " HttpOnly; SameSite=Lax; Path=/; Max-Age=600");
            assert_eq!(
                location(&response),
                Some(format!("{authorize}{nonce}.7").as_str())
            );
            nonces.push(nonce.to_owned());
        }
    }
    nonces.sort();
    nonces.dedup();
    assert_eq!(nonces.len(), 4, "every connection gets its own nonce");

    let head = app
        .send(
            Method::HEAD,
            "/patreon/connect?guild=8",
            &[("cookie", "session=admin-token")],
            Vec::new(),
        )
        .await
        .unwrap();
    assert_eq!(head.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        location(&head),
        Some(outcome_location("patreon", "8", "forbidden").as_str())
    );
}

fn spent(flow: &str) -> String {
    format!("{flow}_oauth_state=; Path=/; Max-Age=0")
}

#[sqlx::test(migrations = "../migrations")]
async fn the_callback_routes_check_the_nonce_then_the_caller(pool: PgPool) {
    let app = standard(pool.clone()).await.unwrap();
    let bare = unconfigured(pool.clone()).await.unwrap();
    sign_in(&pool).await.unwrap();

    for (flow, _) in FLOWS {
        let session = "session=admin-token";
        let remembered = format!("{session}; {flow}_oauth_state=nonce");
        let callback = |query: &str| format!("/{flow}/callback{query}");

        for query in ["", "?code=c", "?code=c&state=nodot"] {
            let response = app
                .get(
                    &callback(query),
                    Some(&format!("{session}; {flow}_oauth_state=n")),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SEE_OTHER, "{flow} {query}");
            assert_eq!(location(&response), Some("/guilds"), "{flow} {query}");
            let cookies = set_cookies(&response);
            assert_eq!(cookies.len(), 1, "{flow} {query}");
            assert!(cookies[0].starts_with(&spent(flow)), "{cookies:?}");
        }

        let mismatch = outcome_location(flow, "7", "state_mismatch");
        let wrong = format!("{session}; {flow}_oauth_state=other");
        let empty = format!("{session}; {flow}_oauth_state=");
        for (what, cookie, query) in [
            ("no cookie", session.to_owned(), "?code=c&state=nonce.7"),
            ("other nonce", wrong, "?code=c&state=nonce.7"),
            ("empty cookie", empty.clone(), "?code=c&state=nonce.7"),
            ("empty nonce", empty, "?code=c&state=.7"),
        ] {
            let response = app.get(&callback(query), Some(&cookie)).await.unwrap();
            assert_eq!(response.status(), StatusCode::SEE_OTHER, "{what}");
            assert_eq!(location(&response), Some(mismatch.as_str()), "{what}");
        }

        let response = app
            .get(&callback("?error=access_denied&state=nonce.7"), Some(&remembered))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            location(&response),
            Some(outcome_location(flow, "7", "declined").as_str())
        );
        let cookies = set_cookies(&response);
        assert_eq!(cookies.len(), 1);
        assert!(cookies[0].starts_with(&spent(flow)), "{cookies:?}");

        let response = bare
            .get(&callback("?code=c&state=nonce.7"), Some(&remembered))
            .await
            .unwrap();
        assert_eq!(
            location(&response),
            Some(outcome_location(flow, "7", "unconfigured").as_str())
        );

        for (token, guild) in [("member-token", "7"), ("admin-token", "8")] {
            let cookie = format!("session={token}; {flow}_oauth_state=nonce");
            let response = app
                .get(
                    &callback(&format!("?code=c&state=nonce.{guild}")),
                    Some(&cookie),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                location(&response),
                Some(outcome_location(flow, guild, "forbidden").as_str()),
                "{token} {guild}"
            );
        }

        let response = app
            .get(&callback("?code=c&state=nonce.7.extra"), Some(&remembered))
            .await
            .unwrap();
        assert_eq!(
            location(&response),
            Some(outcome_location(flow, "7.extra", "forbidden").as_str()),
            "the state splits at its first dot"
        );
    }
}
