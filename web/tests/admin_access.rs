//! Every admin call authorizes itself from the request's session: the role
//! checks read `false` for visitors, and each guarded call refuses with the
//! `unauthenticated` / `forbidden` text the pages show before touching
//! Discord or the loadout tables.

use std::error::Error;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;

use http_body_util::BodyExt;
use jiff::{SignedDuration, Timestamp};
use sqlx::PgPool;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::Request;
use topcoat::router::{Body, Router, header, route};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use web::admin::{self, AdminError, EmojiSource, emoji_upload};
use web::auth::AuthError;
use web::state::{UserGuildsCache, WebState};
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Seeded with both dashboard roles by the migrations.
const OPERATOR: i64 = 211_486_447_369_322_506;
const OPERATOR_COOKIE: &str = "session=operator-token";
const PLAIN_COOKIE: &str = "session=plain-token";
const ADMIN_ONLY_COOKIE: &str = "session=admin-only-token";
const OPERATOR_ONLY_COOKIE: &str = "session=operator-only-token";
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

fn shown<T: Debug>(result: std::result::Result<T, AdminError>) -> String {
    match result {
        Ok(value) => format!("ok {value:?}"),
        Err(e) if e.is_denied() => format!("denied {e}"),
        Err(e) => format!("error {e}"),
    }
}

fn shown_auth<T: Debug>(result: std::result::Result<T, AuthError>) -> String {
    shown(result.map_err(AdminError::from))
}

#[route(GET "/t/roles")]
async fn roles(cx: &Cx) -> Result<String> {
    Ok(format!(
        "{} {}",
        shown_auth(admin::is_admin(cx).await),
        shown_auth(admin::is_operator(cx).await)
    ))
}

#[route(GET "/t/operator-access")]
async fn operator_access(cx: &Cx) -> Result<String> {
    Ok(shown_auth(admin::guild_operator_access(cx, "7").await))
}

#[route(GET "/t/operator-access-bad-id")]
async fn operator_access_bad_id(cx: &Cx) -> Result<String> {
    Ok(shown_auth(admin::guild_operator_access(cx, "seven").await))
}

#[route(GET "/t/servers")]
async fn servers(cx: &Cx) -> Result<String> {
    Ok(shown(admin::list_bot_guilds(cx).await.map(|g| g.len())))
}

#[route(GET "/t/loadouts")]
async fn loadouts(cx: &Cx) -> Result<String> {
    Ok(shown(admin::list_loadouts(cx).await.map(|l| l.is_empty())))
}

#[route(GET "/t/loadout-missing")]
async fn loadout_missing(cx: &Cx) -> Result<String> {
    Ok(shown(admin::get_loadout(cx, 987_654).await))
}

#[route(GET "/t/catalog")]
async fn catalog(cx: &Cx) -> Result<String> {
    Ok(shown(
        admin::loadout_catalog(cx)
            .await
            .map(|c| (c.emojis.len(), c.blank == admin::blank())),
    ))
}

#[route(GET "/t/check")]
async fn check(cx: &Cx) -> Result<String> {
    Ok(shown(admin::check_loadout(cx, &admin::blank()).await.map(|c| c.error)))
}

#[route(GET "/t/save-blank")]
async fn save_blank(cx: &Cx) -> Result<String> {
    Ok(shown(admin::save_loadout(cx, &admin::blank()).await))
}

#[route(GET "/t/delete-missing")]
async fn delete_missing(cx: &Cx) -> Result<String> {
    Ok(shown(admin::delete_loadout(cx, 987_654).await))
}

async fn create(cx: &Cx, name: &str, source: EmojiSource) -> String {
    shown(admin::create_zayden_emoji(cx, name.to_owned(), source).await)
}

#[route(GET "/t/emoji")]
async fn emoji(cx: &Cx) -> Result<String> {
    let png = emoji_upload::data_uri(PNG)?;
    Ok([
        create(cx, "Bad Name", EmojiSource::DataUri(png.clone())).await,
        create(cx, "auto_rifle", EmojiSource::DataUri(png.clone())).await,
        create(cx, "new_emoji", EmojiSource::Url("http://example.com/a.png".into()))
            .await,
        create(cx, "new_emoji", EmojiSource::DataUri("data:image/png,raw".into()))
            .await,
        create(
            cx,
            "new_emoji",
            EmojiSource::DataUri("data:text/plain;base64,aGk=".into()),
        )
        .await,
        create(cx, "new_emoji", EmojiSource::DataUri(png)).await,
    ]
    .join("\n"))
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
        zayden_id: 0,
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

struct Harness {
    router: Router,
    user_guilds: UserGuildsCache,
}

/// A router whose state never reaches Discord: `zayden_id` is 0, and the
/// calls under test refuse before any bot request.
fn harness(pool: &PgPool) -> TestResult<Harness> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let config = config();
    let app = Arc::new(ZaydenAppState::new(pool.clone(), &config));
    let state = WebState::new(app, &config)?;
    let user_guilds = state.discord.user_guilds.clone();

    let base = Router::builder()
        .app_context(state)
        .route(roles)
        .route(operator_access)
        .route(operator_access_bad_id)
        .route(servers)
        .route(loadouts)
        .route(loadout_missing)
        .route(catalog)
        .route(check)
        .route(save_blank)
        .route(delete_missing)
        .route(emoji);

    Ok(Harness { router: web::router(base), user_guilds })
}

impl Harness {
    async fn text(&self, path: &str, cookie: Option<&str>) -> TestResult<String> {
        let mut request = Request::get(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let response = self.router.handle(request.body(Body::empty())?).await;
        let bytes = response.into_body().collect().await?.to_bytes();
        Ok(String::from_utf8(bytes.to_vec())?)
    }
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

async fn grant(pool: &PgPool, user_id: i64, role: &str) -> TestResult<()> {
    sqlx::query!(
        "INSERT INTO web_user_roles (discord_user_id, role) VALUES ($1, $2)",
        user_id,
        role
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn seeded(pool: &PgPool) -> TestResult<Harness> {
    insert_session(pool, "operator-token", OPERATOR).await?;
    insert_session(pool, "plain-token", 43).await?;
    insert_session(pool, "admin-only-token", 44).await?;
    grant(pool, 44, "admin").await?;
    insert_session(pool, "operator-only-token", 45).await?;
    grant(pool, 45, "operator").await?;
    harness(pool)
}

#[sqlx::test(migrations = "../migrations")]
async fn role_checks_read_false_for_visitors_and_plain_users(pool: PgPool) {
    let app = seeded(&pool).await.unwrap();

    assert_eq!(app.text("/t/roles", None).await.unwrap(), "ok false ok false");
    assert_eq!(
        app.text("/t/roles", Some("session=unknown")).await.unwrap(),
        "ok false ok false"
    );
    assert_eq!(
        app.text("/t/roles", Some(PLAIN_COOKIE)).await.unwrap(),
        "ok false ok false"
    );
    assert_eq!(
        app.text("/t/roles", Some(OPERATOR_COOKIE)).await.unwrap(),
        "ok true ok true"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn operator_access_is_only_for_operators_without_member_rights(pool: PgPool) {
    let app = seeded(&pool).await.unwrap();
    app.user_guilds
        .insert(
            OPERATOR,
            Arc::from([CurrentUserGuild {
                id: Id::new(7),
                name: "Guild 7".to_owned(),
                icon: None,
                owner: false,
                permissions: Permissions::MANAGE_GUILD,
                features: Vec::new(),
            }]),
        )
        .await;

    assert_eq!(app.text("/t/operator-access", None).await.unwrap(), "ok false");
    assert_eq!(
        app.text("/t/operator-access", Some(PLAIN_COOKIE)).await.unwrap(),
        "ok false"
    );
    assert_eq!(
        app.text("/t/operator-access", Some(OPERATOR_COOKIE)).await.unwrap(),
        "ok false"
    );
    assert_eq!(
        app.text("/t/operator-access-bad-id", Some(OPERATOR_COOKIE)).await.unwrap(),
        "error invalid guild id"
    );
    assert_eq!(
        app.text("/t/operator-access-bad-id", Some(PLAIN_COOKIE)).await.unwrap(),
        "ok false"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn guarded_calls_refuse_visitors_and_users_without_the_role(pool: PgPool) {
    let app = seeded(&pool).await.unwrap();

    for path in [
        "/t/servers",
        "/t/loadouts",
        "/t/loadout-missing",
        "/t/catalog",
        "/t/check",
        "/t/save-blank",
        "/t/delete-missing",
    ] {
        assert_eq!(
            app.text(path, None).await.unwrap(),
            "denied unauthenticated",
            "{path}"
        );
        assert_eq!(
            app.text(path, Some(PLAIN_COOKIE)).await.unwrap(),
            "denied forbidden",
            "{path}"
        );
    }

    let refusals = app.text("/t/emoji", Some(PLAIN_COOKIE)).await.unwrap();
    assert!(refusals.lines().all(|line| line == "denied forbidden"), "{refusals}");
}

#[sqlx::test(migrations = "../migrations")]
async fn admins_reach_the_loadout_calls(pool: PgPool) {
    let app = seeded(&pool).await.unwrap();
    let as_admin = |path| app.text(path, Some(OPERATOR_COOKIE));

    assert_eq!(as_admin("/t/loadouts").await.unwrap(), "ok false");
    assert_eq!(
        as_admin("/t/loadout-missing").await.unwrap(),
        "error loadout 987654 does not exist"
    );
    assert_eq!(as_admin("/t/catalog").await.unwrap(), "ok (0, true)");
    assert_eq!(as_admin("/t/check").await.unwrap(), "ok Some(\"name is required\")");
    assert_eq!(as_admin("/t/save-blank").await.unwrap(), "error name is required");
    assert_eq!(
        as_admin("/t/delete-missing").await.unwrap(),
        "error loadout 987654 does not exist"
    );
}

/// `admin` and `operator` are independent: neither grants the other's calls.
#[sqlx::test(migrations = "../migrations")]
async fn each_role_opens_only_its_own_calls(pool: PgPool) {
    let app = seeded(&pool).await.unwrap();

    assert_eq!(
        app.text("/t/roles", Some(ADMIN_ONLY_COOKIE)).await.unwrap(),
        "ok true ok false"
    );
    assert_eq!(
        app.text("/t/servers", Some(ADMIN_ONLY_COOKIE)).await.unwrap(),
        "denied forbidden"
    );
    assert_eq!(
        app.text("/t/loadouts", Some(ADMIN_ONLY_COOKIE)).await.unwrap(),
        "ok false"
    );

    assert_eq!(
        app.text("/t/roles", Some(OPERATOR_ONLY_COOKIE)).await.unwrap(),
        "ok false ok true"
    );
    for path in [
        "/t/loadouts",
        "/t/loadout-missing",
        "/t/catalog",
        "/t/check",
        "/t/save-blank",
        "/t/delete-missing",
    ] {
        assert_eq!(
            app.text(path, Some(OPERATOR_ONLY_COOKIE)).await.unwrap(),
            "denied forbidden",
            "{path}"
        );
    }
    let refusals = app.text("/t/emoji", Some(OPERATOR_ONLY_COOKIE)).await.unwrap();
    assert!(refusals.lines().all(|line| line == "denied forbidden"), "{refusals}");
}

/// The checks run in order and stop before Discord: name, reserved icon
/// keys, then the image, then the bot's application id.
#[sqlx::test(migrations = "../migrations")]
async fn emoji_creation_checks_the_name_and_image_first(pool: PgPool) {
    let app = seeded(&pool).await.unwrap();

    let results = app.text("/t/emoji", Some(OPERATOR_COOKIE)).await.unwrap();

    assert_eq!(results.lines().collect::<Vec<_>>(), [
        "error emoji names are 2-32 lowercase letters, digits or underscores",
        "error `auto_rifle` is reserved for a built-in class, element, weapon or stat icon",
        "error the image link must be an https:// URL",
        "error the chosen file couldn't be read",
        "error images must be PNG, JPEG, GIF or WebP",
        "error zayden_id is not configured",
    ]);
}
