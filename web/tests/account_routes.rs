//! The linked-accounts page and the Steam link flow through the app router,
//! against Postgres. Sessions come from a seeded cache, so no case reaches
//! Discord or Steam.

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use accounts::links::AccountLink;
use accounts::provider::Provider;
use deadlock::db::{guild_members, user_settings};
use http_body_util::BodyExt;
use secrecy::SecretString;
use sqlx::PgPool;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, StatusCode, header};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use web::account::deadlock::shared_selection;
use web::auth::safe_next;
use web::components::brand::LOGO;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::state::{SessionIdentity, WebState};
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const MEMBER: &str = "session=member-token";
const USER_ID: i64 = 41;
const STEAM_ID: &str = "76561197960287930";
const FORM: &str = "application/x-www-form-urlencoded";
const SERVED_GUILD: i64 = 7;
const UNSERVED_GUILD: i64 = 8;

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-account-assets-{}", std::process::id()));
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
        upgrade_url: None,
        kofi_verification_token: None,
        patreon: None,
        youtube: None,
        discord_sku_pro: None,
        discord_sku_ultra: None,
        radio_stations: Arc::from(Vec::new()),
    }
}

fn member_guild(id: i64, name: &str) -> CurrentUserGuild {
    CurrentUserGuild {
        id: Id::new(id.cast_unsigned()),
        name: name.to_owned(),
        icon: None,
        owner: false,
        permissions: Permissions::empty(),
        features: Vec::new(),
    }
}

async fn seed_served_guild(pool: &PgPool) -> TestResult {
    sqlx::query!("INSERT INTO guilds (id) VALUES ($1)", SERVED_GUILD)
        .execute(pool)
        .await?;
    sqlx::query!(
        "INSERT INTO guild_presence (guild_id, application_id) VALUES ($1, 1)",
        SERVED_GUILD
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn harness(pool: &PgPool) -> TestResult<Router> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let config = config();
    let app = Arc::new(ZaydenAppState::new(pool.clone(), &config));
    let state = WebState::new(app, &config)?;
    state
        .sessions
        .insert("member-token".to_owned(), SessionIdentity {
            user_id: USER_ID,
            access_token: "member-token-access".to_owned(),
        })
        .await;
    state
        .discord
        .user_guilds
        .insert(
            USER_ID,
            Arc::from([
                member_guild(SERVED_GUILD, "Served"),
                member_guild(UNSERVED_GUILD, "Unserved"),
            ]),
        )
        .await;

    let base = Router::builder()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .app_context(state);
    Ok(web::router(base))
}

async fn get(
    router: &Router,
    path: &str,
    cookie: Option<&str>,
) -> TestResult<Response> {
    let mut request = Request::get(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    Ok(router.handle(request.body(Body::empty())?).await)
}

async fn post(
    router: &Router,
    path: &str,
    cookie: Option<&str>,
) -> TestResult<Response> {
    let mut request = Request::post(path).header(header::CONTENT_TYPE, FORM);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    Ok(router.handle(request.body(Body::empty())?).await)
}

async fn post_form(
    router: &Router,
    path: &str,
    cookie: &str,
    body: &str,
) -> TestResult<Response> {
    let request = Request::post(path)
        .header(header::CONTENT_TYPE, FORM)
        .header(header::COOKIE, cookie)
        .body(Body::from(body.to_owned()))?;
    Ok(router.handle(request).await)
}

async fn body_text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn location(response: &Response) -> Option<&str> {
    response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok())
}

fn set_cookies(response: &Response) -> Vec<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_owned))
        .collect()
}

#[sqlx::test(migrations = "../migrations")]
async fn a_signed_out_visitor_is_sent_to_discord_and_remembered(pool: PgPool) {
    let router = harness(&pool).await.unwrap();

    for (path, next) in [
        ("/account/link/steam", "%2Faccount%2Flink%2Fsteam"),
        ("/account/link/steam/callback", "%2Faccount"),
        ("/account", "%2Faccount"),
    ] {
        let response = get(&router, path, None).await.unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(location(&response), Some("/auth/discord"), "{path}");
        assert!(
            set_cookies(&response)
                .iter()
                .any(|cookie| cookie.starts_with(&format!("login_next={next};"))),
            "{path}: {:?}",
            set_cookies(&response)
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn a_signed_in_user_is_sent_to_steam_with_a_fresh_nonce(pool: PgPool) {
    let router = harness(&pool).await.unwrap();

    let response = get(&router, "/account/link/steam", Some(MEMBER)).await.unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let target = location(&response).unwrap();
    assert!(
        target.starts_with("https://steamcommunity.com/openid/login?"),
        "{target}"
    );
    assert!(target.contains("openid.mode=checkid_setup"), "{target}");
    assert!(
        target.contains(
            "http%3A%2F%2Flocalhost%3A3000%2Faccount%2Flink%2Fsteam%2Fcallback%3Fn%3D"
        ),
        "{target}"
    );

    let cookies = set_cookies(&response);
    let nonce = cookies
        .iter()
        .find_map(|cookie| cookie.strip_prefix("steam_openid_nonce="))
        .and_then(|rest| rest.split_once(';'))
        .map(|(nonce, _)| nonce)
        .unwrap();
    assert_ne!(nonce, "");
    assert!(target.contains(&format!("%3Fn%3D{nonce}")), "{target}");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_callback_with_a_wrong_nonce_links_nothing(pool: PgPool) {
    let router = harness(&pool).await.unwrap();

    let response = get(
        &router,
        &format!(
            "/account/link/steam/callback?n=wrong&openid.mode=id_res&openid.claimed_id=\
             https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Fid%2F{STEAM_ID}"
        ),
        Some(MEMBER),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/account"));
    assert!(AccountLink::list_for_user(&pool, USER_ID).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn a_cancelled_steam_sign_in_returns_to_the_page(pool: PgPool) {
    let router = harness(&pool).await.unwrap();

    let response = get(
        &router,
        "/account/link/steam/callback?n=abc&openid.mode=cancel",
        Some(&format!("{MEMBER}; steam_openid_nonce=abc")),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/account"));
    assert!(AccountLink::list_for_user(&pool, USER_ID).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn the_account_page_lists_the_linked_steam_account(pool: PgPool) {
    let router = harness(&pool).await.unwrap();
    AccountLink::upsert_transfer(&pool, USER_ID, Provider::Steam, STEAM_ID)
        .await
        .unwrap();

    let response = get(&router, "/account", Some(MEMBER)).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let html = body_text(response).await.unwrap();
    assert!(html.contains("[U:1:22202]"), "{html}");
    assert!(html.contains(STEAM_ID), "{html}");
    assert!(html.contains(r#"action="/account/unlink/steam""#), "{html}");
    assert!(!html.contains("/account/link/steam"), "{html}");
}

#[sqlx::test(migrations = "../migrations")]
async fn the_account_page_offers_linking_when_nothing_is_linked(pool: PgPool) {
    let router = harness(&pool).await.unwrap();

    let response = get(&router, "/account", Some(MEMBER)).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"href="/account/link/steam""#), "{html}");
    assert!(html.contains("Link Steam"), "{html}");
}

#[sqlx::test(migrations = "../migrations")]
async fn unlinking_removes_the_row_and_returns_to_the_page(pool: PgPool) {
    let router = harness(&pool).await.unwrap();
    AccountLink::upsert_transfer(&pool, USER_ID, Provider::Steam, STEAM_ID)
        .await
        .unwrap();

    let response =
        post(&router, "/account/unlink/steam", Some(MEMBER)).await.unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/account"));
    assert!(AccountLink::list_for_user(&pool, USER_ID).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn an_unknown_provider_unlinks_nothing(pool: PgPool) {
    let router = harness(&pool).await.unwrap();
    AccountLink::upsert_transfer(&pool, USER_ID, Provider::Steam, STEAM_ID)
        .await
        .unwrap();

    let response =
        post(&router, "/account/unlink/epic", Some(MEMBER)).await.unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/account"));
    assert_eq!(AccountLink::list_for_user(&pool, USER_ID).await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn the_account_page_offers_summaries_for_servers_that_use_the_module(
    pool: PgPool,
) {
    let router = harness(&pool).await.unwrap();
    seed_served_guild(&pool).await.unwrap();

    let response = get(&router, "/account", Some(MEMBER)).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(r#"action="/account/deadlock""#), "{html}");
    assert!(html.contains(r#"action="/account/deadlock/servers""#), "{html}");
    assert!(html.contains(r#"name="dm_summaries""#), "{html}");
    assert!(html.contains(r#"name="share_7""#), "{html}");
    assert!(!html.contains(r#"name="share_8""#), "{html}");
}

#[sqlx::test(migrations = "../migrations")]
async fn the_dm_checkbox_saves_and_clears_the_preference(pool: PgPool) {
    let router = harness(&pool).await.unwrap();

    let response =
        post_form(&router, "/account/deadlock", MEMBER, "").await.unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/account"));
    assert!(!user_settings::dm_summaries(&pool, USER_ID).await.unwrap());

    let response =
        post_form(&router, "/account/deadlock", MEMBER, "dm_summaries=on")
            .await
            .unwrap();

    assert_eq!(location(&response), Some("/account"));
    assert!(user_settings::dm_summaries(&pool, USER_ID).await.unwrap());
}

#[sqlx::test(migrations = "../migrations")]
async fn sharing_is_saved_only_for_servers_that_use_the_module(pool: PgPool) {
    let router = harness(&pool).await.unwrap();
    seed_served_guild(&pool).await.unwrap();

    let response = post_form(
        &router,
        "/account/deadlock/servers",
        MEMBER,
        "share_7=on&share_8=on&share_9=on",
    )
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/account"));
    assert_eq!(guild_members::shared_guilds(&pool, USER_ID).await.unwrap(), vec![
        SERVED_GUILD
    ]);

    post_form(&router, "/account/deadlock/servers", MEMBER, "").await.unwrap();

    assert_eq!(
        guild_members::shared_guilds(&pool, USER_ID).await.unwrap(),
        Vec::<i64>::new()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_signed_out_visitor_cannot_change_summary_settings(pool: PgPool) {
    let router = harness(&pool).await.unwrap();

    for path in ["/account/deadlock", "/account/deadlock/servers"] {
        let response = post(&router, path, None).await.unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(location(&response), Some("/auth/discord"), "{path}");
    }
    assert!(user_settings::dm_summaries(&pool, USER_ID).await.unwrap());
}

#[test]
fn the_selection_covers_exactly_the_allowed_servers() {
    let form = vec![
        ("share_7".to_owned(), "on".to_owned()),
        ("share_9".to_owned(), "on".to_owned()),
        ("dm_summaries".to_owned(), "on".to_owned()),
    ];

    assert_eq!(shared_selection(&[7, 8], &form), vec![(7, true), (8, false)]);
}

#[test]
fn an_empty_form_clears_every_allowed_server() {
    let form = [("share_7".to_owned(), "on".to_owned())];

    assert_eq!(shared_selection(&[7, 8], &[]), vec![(7, false), (8, false)]);
    assert_eq!(shared_selection(&[], &form), Vec::<(i64, bool)>::new());
}

#[test]
fn only_local_absolute_paths_are_accepted_as_a_next_target() {
    for path in ["/account", "/account/link/steam"] {
        assert_eq!(safe_next(path), Some(path));
    }

    for path in ["", "account", "//evil.com", "/\\evil", "/a b", "https://x"] {
        assert_eq!(safe_next(path), None, "{path:?}");
    }
}
