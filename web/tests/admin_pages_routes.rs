//! The operator server list and the loadout list through the app router,
//! against Postgres.
//!
//! Sessions come from a seeded cache and roles from `web_user_roles`, so no
//! case reaches Discord: the operator list needs the bot's guild list, so its
//! signed-in markup is covered in `admin_pages_markup`.

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use destiny2::endgame_analysis::sheet::Affinity;
use destiny2::loadouts::{
    Archetype,
    ArmourSlot,
    Class,
    Element,
    Mode,
    RawArmour,
    RawAspect,
    RawLoadout,
    RawWeapon,
    StatKind,
};
use http_body_util::BodyExt;
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, StatusCode, header};
use web::admin::pages::{LOADOUTS_TITLE, SERVERS_TITLE};
use web::admin::{draft, loadout_form, summaries, write_unchecked};
use web::auth::{SessionUser, WebRole, has_role};
use web::components::brand::LOGO;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::state::{SessionIdentity, SessionUsersCache, WebState};
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const MEMBER: &str = "session=member-token";
const ADMIN: &str = "session=admin-token";
const FORM: &str = "application/x-www-form-urlencoded";
const LIST: &str = "/admin/destiny2/loadouts";

/// `sqlx::test` lends every test pool's connections out of one 20-permit
/// master pool, and a pool keeps each permit it took until the pool is
/// dropped, so each test pool is capped at two connections.
const TEST_POOL_CONNECTIONS: u32 = 2;

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-admin-pages-assets-{}", std::process::id()));
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

async fn test_pool(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) -> TestResult<PgPool> {
    Ok(options.max_connections(TEST_POOL_CONNECTIONS).connect_with(connect).await?)
}

struct Harness {
    router: Router,
}

async fn harness(pool: &PgPool) -> TestResult<Harness> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let config = config();
    let app = Arc::new(ZaydenAppState::new(pool.clone(), &config));
    let state = WebState::new(app, &config)?;
    for (token, user_id) in [("member-token", 41), ("admin-token", 50)] {
        state
            .sessions
            .insert(token.to_owned(), SessionIdentity {
                user_id,
                access_token: format!("{token}-access"),
            })
            .await;
    }
    for user_id in [41, 50] {
        state.discord.user_guilds.insert(user_id, Arc::from([])).await;
    }
    seed_users(&state.discord.users, &[41, 50]).await;

    let base = Router::builder()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .app_context(state);
    Ok(Harness { router: web::router(base) })
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

    async fn page(&self, path: &str, cookie: Option<&str>) -> TestResult<String> {
        Ok(normalize(&self.raw_page(path, cookie).await?))
    }

    async fn raw_page(
        &self,
        path: &str,
        cookie: Option<&str>,
    ) -> TestResult<String> {
        let response = self.get(path, cookie).await?;
        if response.status() != StatusCode::OK {
            return Err(format!("{path} answered {}", response.status()).into());
        }
        let bytes = response.into_body().collect().await?.to_bytes();
        Ok(String::from_utf8(bytes.to_vec())?)
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

fn location(response: &Response) -> Option<&str> {
    response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok())
}

async fn grant_role(pool: &PgPool, user_id: i64, role: &str) -> TestResult {
    sqlx::query!(
        "INSERT INTO web_user_roles (discord_user_id, role) VALUES ($1, $2)",
        user_id,
        role
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn raw(name: &str, class: Class, element: Element) -> RawLoadout {
    RawLoadout {
        name: name.into(),
        class,
        element,
        mode: Mode::PvE,
        tags: vec!["Raid".into()],
        super_name: "Song of Flame".into(),
        super_emoji: "song_of_flame".into(),
        class_ability: "healing_rift".into(),
        jump: "burst_glide".into(),
        melee: "arcane_needle".into(),
        grenade: "storm_grenade".into(),
        aspects: vec![RawAspect {
            aspect: "feed_the_void".into(),
            fragments: vec!["facet_of_dawn".into()],
        }],
        weapons: vec![RawWeapon {
            name: "Pages Test Rifle".into(),
            affinity: Affinity::Solar,
            archetype: Archetype::PulseRifle,
            icon_url: "https://www.bungie.net/p.jpg".into(),
            perks: vec!["pages_test_perk".into()],
        }],
        armour: vec![RawArmour {
            slot: ArmourSlot::Chest,
            name: "Pages Test Robes".into(),
            icon_url: "https://www.bungie.net/r.jpg".into(),
            mods: vec!["recuperation".into()],
        }],
        stats: vec![(StatKind::Grenade, 200), (StatKind::Health, 70)],
        artifact_name: "Pages Artifact".into(),
        artifact_perks: vec!["radiant_shrapnel".into()],
        author: "Oscar".into(),
        dim_link: "https://dim.gg/pages".into(),
        video_url: String::new(),
        how_it_works: "Throw storm grenades.".into(),
    }
}

async fn save(pool: &PgPool, raw: RawLoadout) -> TestResult<i32> {
    let form = loadout_form(None, raw);
    Ok(write_unchecked(pool, None, &draft(&form)?).await?)
}

fn confirm(id: i32, name: &str) -> String {
    format!(
        r#"<div class="confirm" data-confirm=""><button type="submit" class="btn btn-ghost" data-confirm-trigger="">Delete</button><dialog class="dialog" aria-labelledby="loadout-{id}-delete-title" aria-describedby="loadout-{id}-delete-desc" data-confirm-dialog=""><div class="dialog-panel"><h2 class="dialog-title" id="loadout-{id}-delete-title">Delete loadout “{name}”?</h2><p class="dialog-desc" id="loadout-{id}-delete-desc">This removes the build from /destiny2 builds for everyone. It cannot be undone.</p><div class="dialog-actions"><button type="button" class="btn btn-secondary" data-dialog-close="" autofocus="">Cancel</button><button type="submit" class="btn btn-danger">Delete loadout</button></div></div></dialog></div>"#
    )
}

fn row(id: i32, name: &str, cells: [&str; 4]) -> String {
    let [class, element, mode, author] = cells;
    format!(
        r#"<tr role="row"><th scope="row" role="rowheader" data-label="Loadout"><a href="{LIST}/{id}" class="loadout-name">{name}</a></th><td role="cell" data-label="Class">{class}</td><td role="cell" data-label="Subclass">{element}</td><td role="cell" data-label="Mode">{mode}</td><td role="cell" data-label="Author">{author}</td><td role="cell" data-label="Action"><form method="post" action="{LIST}" data-pending=""><input type="hidden" name="id" value="{id}">{}</form></td></tr>"#,
        confirm(id, name)
    )
}

const HEADER: &str = r#"<div class="page"><div class="page-header"><div><h1>Loadout builder</h1><p class="page-lead">Builds shown by /destiny2 builds. Saves apply to the bot immediately.</p></div><a href="/admin/destiny2/loadouts/new" class="btn btn-primary">New loadout</a></div>"#;
const NO_FLASH: &str = r#"<div class="flash-region" role="status"></div>"#;
const SKELETONS: &str = r#"<div class="skeleton-list"><div class="skeleton-row" aria-hidden="true"></div><div class="skeleton-row" aria-hidden="true"></div><div class="skeleton-row" aria-hidden="true"></div><div class="skeleton-row" aria-hidden="true"></div><div class="skeleton-row" aria-hidden="true"></div><div class="skeleton-row" aria-hidden="true"></div></div>"#;
const FILTER: &str = r#"<div class="field-row"><label class="field-label" for="loadout-class-filter">Class</label><div class="select"><select class="input loadout-filter" id="loadout-class-filter"><option value="">All classes</option><option value="Hunter">Hunter</option><option value="Titan">Titan</option><option value="Warlock">Warlock</option></select><span class="select-chevron"><svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></span></div></div>"#;
const TABLE_HEAD: &str = r#"<div class="data-table-wrap"><table class="data-table" role="table"><caption class="visually-hidden">Loadouts</caption><thead role="rowgroup"><tr role="row"><th scope="col" role="columnheader">Loadout</th><th scope="col" role="columnheader">Class</th><th scope="col" role="columnheader">Subclass</th><th scope="col" role="columnheader">Mode</th><th scope="col" role="columnheader">Author</th><th scope="col" role="columnheader">Action</th></tr></thead><tbody role="rowgroup">"#;

/// The 403 page a signed-in viewer without the role gets.
fn forbidden(who: &str) -> String {
    format!(
        r#"<main id="main" class="app-main" tabindex="-1"><div class="page"><section class="error-panel"><h1 class="error-title">You don't have access to this page</h1><p class="error-text">This page is for Zayden's {who}.</p><div class="error-actions"><a href="/guilds" class="btn btn-primary">Back to servers</a></div></section></div></main>"#
    )
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

#[sqlx::test(migrations = "../migrations")]
async fn the_server_list_sends_visitors_to_sign_in_and_refuses_users_without_the_role(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    grant_role(&pool, 50, "admin").await.unwrap();
    let app = harness(&pool).await.unwrap();

    assert!(!has_role(&pool, 41, WebRole::Operator).await.unwrap());
    assert!(has_role(&pool, 50, WebRole::Admin).await.unwrap());
    assert!(!has_role(&pool, 50, WebRole::Operator).await.unwrap());

    for cookie in [None, Some("session=unknown")] {
        let response = app.get("/admin/servers", cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{cookie:?}");
        assert_eq!(location(&response), Some("/login"), "{cookie:?}");
    }
    for cookie in [MEMBER, ADMIN] {
        let response = app.get("/admin/servers", Some(cookie)).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{cookie}");
        let html = body_text(response).await.unwrap();
        assert!(html.contains(&forbidden("operators")), "{cookie}: {html}");
        assert!(!html.contains("operator-tools"), "{html}");
        assert!(
            html.contains("<title>All bot servers - Zayden Dashboard</title>"),
            "{html}"
        );
        assert!(!html.contains(r#"href="/admin/servers""#), "{html}");
    }
    assert_eq!(SERVERS_TITLE, "All bot servers - Zayden Dashboard");

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn the_loadout_list_shows_a_filterable_table_and_refuses_without_the_role(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    grant_role(&pool, 50, "admin").await.unwrap();
    let app = harness(&pool).await.unwrap();

    let before = summaries(&pool).await.unwrap().len();
    let warlock =
        save(&pool, raw("Pages Weavewalk", Class::Warlock, Element::Strand))
            .await
            .unwrap();
    let hunter = save(&pool, raw("Pages Shatter", Class::Hunter, Element::Arc))
        .await
        .unwrap();
    let titan = save(&pool, raw("Pages Ascension", Class::Titan, Element::Solar))
        .await
        .unwrap();

    assert!(has_role(&pool, 50, WebRole::Admin).await.unwrap());
    assert!(!has_role(&pool, 41, WebRole::Admin).await.unwrap());
    let stored = summaries(&pool).await.unwrap();
    assert_eq!(stored.len(), before + 3);
    let position = |id| stored.iter().position(|l| l.id == id).unwrap();
    assert!(
        position(hunter) < position(titan) && position(titan) < position(warlock)
    );

    let html = app.page(LIST, Some(ADMIN)).await.unwrap();
    let rows: String = stored
        .iter()
        .map(|l| row(l.id, &l.name, [&l.class, &l.element, &l.mode, &l.author]))
        .collect();
    let total = stored.len();
    assert!(
        html.contains(&format!(
            r#"<p class="operator-count" role="status">{total} of {total} loadouts</p>{TABLE_HEAD}{rows}</tbody></table></div>"#
        )),
        "{html}"
    );
    assert!(
        html.contains(&format!("{HEADER}{NO_FLASH}{FILTER}{SKELETONS}</div>")),
        "{html}"
    );
    assert!(
        html.contains(&row(hunter, "Pages Shatter", [
            "Hunter", "Arc", "PvE", "Oscar"
        ]))
    );
    assert!(!html.contains("flash-text"), "{html}");
    assert_eq!(html.matches(r#"class="loadout-name""#).count(), stored.len());
    assert!(
        html.contains(
            r#"<a href="/admin/destiny2/loadouts" class="nav-link" aria-current="page">"#
        ),
        "{html}"
    );
    assert!(
        html.contains("<title>Loadout builder - Zayden Dashboard</title>"),
        "{html}"
    );
    assert_eq!(LOADOUTS_TITLE, "Loadout builder - Zayden Dashboard");

    let raw = app.raw_page(LIST, Some(ADMIN)).await.unwrap();
    assert_eq!(raw.matches(" data-topcoat-on:change=").count(), 1, "{raw}");
    assert_eq!(
        raw.matches(r#"<tr role="row" data-topcoat-bind:hidden="#).count(),
        stored.len(),
        "{raw}"
    );
    assert!(raw.contains("String.fromCharCode(10)"), "{raw}");
    assert_eq!(raw.matches(r#"data-pending="""#).count(), stored.len());

    for cookie in [None, Some("session=unknown")] {
        let response = app.get(LIST, cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{cookie:?}");
        assert_eq!(location(&response), Some("/login"), "{cookie:?}");
    }
    let response = app.get(LIST, Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let html = body_text(response).await.unwrap();
    assert!(html.contains(&forbidden("loadout admins")), "{html}");
    assert!(!html.contains("loadout-name"), "{html}");

    for query in
        ["deleted=1", "deleted=1&deleted=1", "deleted=0", "other=1", "%FF=1"]
    {
        let html = app.page(&format!("{LIST}?{query}"), Some(ADMIN)).await.unwrap();
        assert!(!html.contains("Loadout deleted."), "{query}: {html}");
        assert_eq!(html.matches(r#"class="loadout-name""#).count(), stored.len());
    }

    pool.close().await;
}

#[sqlx::test(migrations = "../migrations")]
async fn a_delete_goes_back_to_the_list_or_says_why_not(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) {
    let pool = test_pool(options, connect).await.unwrap();
    grant_role(&pool, 50, "admin").await.unwrap();
    let app = harness(&pool).await.unwrap();

    let keep =
        save(&pool, raw("Pages Keeper", Class::Hunter, Element::Arc)).await.unwrap();
    let gone =
        save(&pool, raw("Pages Goner", Class::Titan, Element::Void)).await.unwrap();
    assert!(has_role(&pool, 50, WebRole::Admin).await.unwrap());
    let before = summaries(&pool).await.unwrap().len();

    let response = app.post(LIST, &format!("id={gone}"), None).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));
    let response =
        app.post(LIST, &format!("id={gone}"), Some(MEMBER)).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(
        body_text(response).await.unwrap().contains(&forbidden("loadout admins"))
    );
    assert_eq!(summaries(&pool).await.unwrap().len(), before);

    let refused = |message: &str| {
        format!(
            r#"{NO_FLASH}<div class="error" id="delete-summary" role="alert" tabindex="-1" autofocus="">Not deleted: {message}</div>"#
        )
    };
    let attempts = [
        ("id=abc".to_owned(), "loadout id `abc` is not a whole number".to_owned()),
        (String::new(), "missing field `id`".to_owned()),
        (format!("id={gone}&id={keep}"), "duplicate field `id`".to_owned()),
        (format!("id={gone}&extra=1"), "unknown field `extra`".to_owned()),
        ("id=987654".to_owned(), "loadout 987654 does not exist".to_owned()),
    ];
    for (body, message) in attempts {
        let response = app.post(LIST, &body, Some(ADMIN)).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let html = body_text(response).await.unwrap();
        assert!(html.contains(&refused(&message)), "{body}: {html}");
        assert!(
            html.contains("<title>Loadout builder - Zayden Dashboard</title>"),
            "{html}"
        );
        assert!(html.contains("<h1>Loadout builder</h1>"), "{html}");
        assert!(!html.contains("error running server function"), "{html}");
        assert_eq!(summaries(&pool).await.unwrap().len(), before, "{body}");
    }

    let response = app.post(LIST, &format!("id={gone}"), Some(ADMIN)).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some(LIST));
    let flash = flash_cookie(&response).unwrap();
    let left = summaries(&pool).await.unwrap();
    assert_eq!(left.len(), before - 1);
    assert!(left.iter().any(|l| l.id == keep) && left.iter().all(|l| l.id != gone));

    let html = app.page(LIST, Some(&format!("{ADMIN}; {flash}"))).await.unwrap();
    assert!(
        html.contains(r#"<div class="flash-region" role="status"><p class="flash flash-success" data-flash="">"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<span class="flash-text">Loadout deleted.</span>"#),
        "{html}"
    );
    assert_eq!(html.matches(r#"class="loadout-name""#).count(), before - 1);
    assert!(
        html.contains("Pages Keeper") && !html.contains("Pages Goner"),
        "{html}"
    );
    let html = app.page(LIST, Some(ADMIN)).await.unwrap();
    assert!(!html.contains("Loadout deleted."), "{html}");

    let response = app.post(LIST, &format!("id={gone}"), Some(ADMIN)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = body_text(response).await.unwrap();
    assert!(
        html.contains(&refused(&format!("loadout {gone} does not exist"))),
        "{html}"
    );

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
