//! The loadout editor page and its JSON endpoints through the app router,
//! against Postgres seeded by the migrations (loadouts 1-11, one admin user).
//!
//! Sessions come from the seeded session cache and `zayden_id` is 0, so no
//! case reaches Discord: the emoji list is empty and emoji creation refuses
//! before any upload.

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, StatusCode, header};
use web::admin::editor::{EDITOR_TITLE, LOADOUT_EDITOR_JS};
use web::admin::keys::display_name;
use web::admin::{LoadoutForm, stored_form};
use web::auth::SessionUser;
use web::components::brand::LOGO;
use web::document::{PENDING_SUBMIT, STYLESHEET};
use web::state::{SessionIdentity, SessionUsersCache, WebState};
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

/// Seeded with the `admin` role by the migrations.
const ADMIN: i64 = 211_486_447_369_322_506;
const ADMIN_COOKIE: &str = "session=admin-token";
const PLAIN_COOKIE: &str = "session=plain-token";
const TEST_POOL_CONNECTIONS: u32 = 2;
const DENIED: &str =
    r#"<p class="error">Admin access is required to edit loadouts.</p>"#;
const SCRIPT_SRC: &str = "/_topcoat/assets/loadout-editor-0123456789abcdef.js";

fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-editor-assets-{}", std::process::id()));
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
        (
            "loadout-editor-0123456789abcdef.js",
            "text/javascript",
            LOADOUT_EDITOR_JS.id().as_u64(),
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
}

async fn harness(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) -> TestResult<(Harness, PgPool)> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let pool =
        options.max_connections(TEST_POOL_CONNECTIONS).connect_with(connect).await?;
    let config = config();
    let app = Arc::new(ZaydenAppState::new(pool.clone(), &config));
    let state = WebState::new(app, &config)?;
    for (token, user_id) in [("admin-token", ADMIN), ("plain-token", 41)] {
        state
            .sessions
            .insert(token.to_owned(), SessionIdentity {
                user_id,
                access_token: format!("{token}-access"),
            })
            .await;
        state.discord.user_guilds.insert(user_id, Arc::from([])).await;
        seed_users(&state.discord.users, &[user_id]).await;
    }
    let base = Router::builder()
        .assets(AssetBundle::load_dir(bundle_dir()?)?)
        .app_context(state);
    Ok((Harness { router: web::router(base) }, pool))
}

impl Harness {
    async fn get(
        &self,
        path: &str,
        cookie: Option<&str>,
    ) -> TestResult<(StatusCode, String)> {
        let mut request = Request::get(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        reply(self.router.handle(request.body(Body::empty())?).await).await
    }

    async fn post_json(
        &self,
        path: &str,
        body: &Value,
        cookie: Option<&str>,
    ) -> TestResult<(StatusCode, Value)> {
        let mut request =
            Request::post(path).header(header::CONTENT_TYPE, "application/json");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let response =
            self.router.handle(request.body(Body::from(body.to_string()))?).await;
        let (status, text) = reply(response).await?;
        Ok((status, serde_json::from_str(&text)?))
    }
}

async fn reply(response: Response) -> TestResult<(StatusCode, String)> {
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok((status, String::from_utf8(bytes.to_vec())?))
}

fn data_block(html: &str) -> TestResult<Value> {
    let start = r#"<script type="application/json" id="loadout-editor-data">"#;
    let (_, rest) = html.split_once(start).ok_or("no data block")?;
    let (json, _) = rest.split_once("</script>").ok_or("unterminated data block")?;
    Ok(serde_json::from_str(json)?)
}

fn form_value(form: &LoadoutForm) -> TestResult<Value> {
    Ok(serde_json::to_value(form)?)
}

fn text(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn attr(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}

fn has(html: &str, needle: &str) -> TestResult {
    if html.contains(needle) {
        Ok(())
    } else {
        Err(format!("missing: {needle}").into())
    }
}

fn ensure(condition: bool, what: &str) -> TestResult {
    if condition { Ok(()) } else { Err(what.into()) }
}

/// The server markup of every filled branch for one stored loadout. The test
/// state lists no Zayden emoji, so filled emoji slots show the "?" fallback.
fn assert_filled_markup(html: &str, form: &LoadoutForm) -> TestResult {
    let key = form.super_emoji.as_str();
    ensure(!key.is_empty(), "seeded loadout has a super")?;
    let shown = display_name(key);
    has(
        html,
        &format!(
            r#"<div class="slot slot-diamond slot-filled"><button type="button" class="slot-button" aria-label="Super: {}, change" title="{} ({})"><span class="slot-missing" title="Zayden has no emoji named {}">?</span></button><button type="button" class="slot-clear" aria-label="Remove Super">"#,
            attr(&shown),
            attr(&shown),
            attr(key),
            attr(key)
        ),
    )?;

    let weapons = form.weapons.len();
    ensure(weapons > 0, "seeded loadout has weapons")?;
    for weapon in &form.weapons {
        has(
            html,
            &format!(
                r#"<div class="slot slot-item slot-filled"><button type="button" class="slot-button" aria-label="Weapon: {}, change" title="{}">"#,
                attr(&weapon.name),
                attr(&weapon.name)
            ),
        )?;
        has(
            html,
            &format!(
                r#"<span class="gear-name">{}</span><span class="gear-meta"><button type="button" class="gear-affinity" aria-expanded="false" aria-label="Damage type: {}, change">{}</button>{}"#,
                text(&weapon.name),
                attr(&weapon.affinity),
                text(&weapon.affinity),
                text(&weapon.archetype)
            ),
        )?;
        has(
            html,
            &format!(
                r#"<button type="button" class="btn btn-ghost gear-remove" aria-label="Remove {}">"#,
                attr(&weapon.name)
            ),
        )?;
    }
    if weapons < 3 {
        has(html, &format!(r#"<span class="gear-meta">{weapons} of 3</span>"#))?;
    }

    let mods_list = form.aspects.len() + weapons;
    let (position, armour) = form
        .armour
        .iter()
        .enumerate()
        .find(|(_, a)| !a.name.is_empty() && !a.mods.is_empty())
        .ok_or("seeded loadout has an armour piece with mods")?;
    has(
        html,
        &format!(
            r#"<span class="gear-name">{}</span><span class="gear-meta">{}</span>"#,
            text(&armour.name),
            text(&armour.slot)
        ),
    )?;
    has(
        html,
        &format!(
            r#"<button type="button" class="slot-clear" aria-label="Remove {}">"#,
            attr(&armour.slot)
        ),
    )?;
    has(
        html,
        &format!(
            r#"<div class="slot-drag" draggable="true" title="Drag, or press Alt + arrow keys, to reorder"><div class="slot slot-filled"><button type="button" class="slot-button" id="reorder-{}-0" aria-label="Mods mod 1: {}, change""#,
            mods_list + position,
            attr(&display_name(armour.mods.first().ok_or("mods")?))
        ),
    )?;
    has(
        html,
        &format!(
            r#"<span class="slot-count" aria-hidden="true">{}/5</span>"#,
            armour.mods.len()
        ),
    )?;

    ensure(!form.artifact_name.is_empty(), "seeded loadout has an artifact")?;
    has(
        html,
        &format!(
            r#"<button type="button" class="artifact-choice" aria-label="Artifact: {}, change">{}</button><button type="button" class="btn btn-ghost">Clear</button>"#,
            attr(&form.artifact_name),
            text(&form.artifact_name)
        ),
    )?;

    let stat = form.stats.first().ok_or("seeded loadout has stats")?;
    has(html, &format!(r#"aria-label="Move {}, priority 1""#, attr(&stat.stat)))?;
    has(
        html,
        &format!(
            r#"<input id="stat-value-0" class="input" type="number" placeholder="0–200" value="{}">"#,
            attr(&stat.value)
        ),
    )?;
    for tag in &form.tags {
        has(
            html,
            &format!(
                r#"<span class="chip"><span class="chip-label">{}</span>"#,
                text(tag)
            ),
        )?;
    }
    has(
        html,
        &format!(
            r#"<input id="loadout-author" class="input" type="text" placeholder="" value="{}">"#,
            attr(&form.author)
        ),
    )?;
    has(
        html,
        &format!(
            "<textarea id=\"loadout-how\" class=\"input\" rows=\"8\">\n{}</textarea>",
            text(&form.how_it_works)
        ),
    )?;
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn visitors_and_non_admins_see_the_inline_refusal(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) -> TestResult {
    let (h, pool) = harness(options, connect).await?;

    for path in [
        "/admin/destiny2/loadouts/new",
        "/admin/destiny2/loadouts/1",
        "/admin/destiny2/loadouts/abc",
    ] {
        for cookie in [None, Some(PLAIN_COOKIE)] {
            let (status, html) = h.get(path, cookie).await?;
            assert_eq!(status, StatusCode::OK, "{path} {cookie:?}");
            assert!(
                html.contains(&format!("<title>{EDITOR_TITLE}</title>")),
                "{path}"
            );
            assert!(
                html.contains(&format!(r#"<div class="page">{DENIED}</div>"#)),
                "{path} {cookie:?}"
            );
            assert!(!html.contains("loadout-editor"), "{path} {cookie:?}");
        }
    }

    let blank = form_value(&web::admin::blank())?;
    for path in ["check", "save"] {
        let url = format!("/admin/destiny2/loadouts/editor/{path}");
        assert_eq!(
            h.post_json(&url, &blank, None).await?,
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({
                    "error": if path == "save" { "error running server function: unauthenticated" } else { "unauthenticated" }
                })
            )
        );
        let (status, body) = h.post_json(&url, &blank, Some(PLAIN_COOKIE)).await?;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            body["error"].as_str().is_some_and(|e| e.ends_with("forbidden")),
            "{body}"
        );
    }
    let emoji =
        json!({ "name": "spark", "source": { "Url": "https://example.com/a.png" } });
    assert_eq!(
        h.post_json(
            "/admin/destiny2/loadouts/editor/emoji",
            &emoji,
            Some(PLAIN_COOKIE)
        )
        .await?,
        (StatusCode::UNPROCESSABLE_ENTITY, json!({ "error": "forbidden" }))
    );

    pool.close().await;
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn admins_get_the_server_rendered_editor(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) -> TestResult {
    let (h, pool) = harness(options, connect).await?;

    for path in ["/admin/destiny2/loadouts/new", "/admin/destiny2/loadouts/abc"] {
        let (status, html) = h.get(path, Some(ADMIN_COOKIE)).await?;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(html.contains(r#"<form class="loadout-editor"><header class="loadout-header"><h1>New loadout</h1>"#), "{path}");
        assert!(html.contains(r#"<input id="loadout-name" class="input" type="text" placeholder="" value="">"#), "{path}");
        assert!(html.contains(r#"<button type="button" role="radio" class="icon-choice-option active" aria-checked="true"><span>Hunter</span></button>"#));
        assert!(html.contains(
            r#"<div class="budget" role="status" aria-live="polite"></div></header>"#
        ));
        assert!(html.contains(
            r#"</header><p class="loadout-warning" role="status">Zayden's emoji list couldn't be loaded, so icons are hidden. You can still edit and save.</p><div class="loadout-board">"#
        ));
        for heading in [
            "subclass-heading",
            "gear-heading",
            "artifact-heading",
            "stats-heading",
            "details-heading",
        ] {
            assert!(
                html.contains(&format!(r#"aria-labelledby="{heading}""#)),
                "{heading}"
            );
        }
        assert!(html.contains(r#"aria-label="Super: empty, choose one""#));
        assert!(
            html.contains(
                r#"<span class="slot-count" aria-hidden="true">0/6</span>"#
            )
        );
        assert!(html.contains(r#"<span class="gear-name gear-empty">Add a weapon</span><span class="gear-meta">0 of 3</span>"#));
        assert!(html.contains(r#"aria-label="Move health, priority 1""#));
        assert!(html.contains(r#"<div class="form-actions"><button type="submit" class="btn btn-primary">Save</button></div></form><dialog class="picker" aria-labelledby="picker-title"></dialog>"#));
        assert!(html.contains(&format!(
            r#"<script type="module" src="{SCRIPT_SRC}"></script>"#
        )));
        let data = data_block(&html)?;
        assert_eq!(data["draftKey"], "zayden:loadout-draft:new");
        assert_eq!(data["form"], form_value(&web::admin::blank())?);
        assert_eq!(data["catalog"]["blank"], data["form"]);
    }

    let stored = stored_form(&pool, 1).await?;
    let (status, html) =
        h.get("/admin/destiny2/loadouts/1", Some(ADMIN_COOKIE)).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<h1>Edit loadout</h1>"));
    assert!(html.contains(&format!(
        r#"id="loadout-name" class="input" type="text" placeholder="" value="{}""#,
        stored.name
    )));
    let data = data_block(&html)?;
    assert_eq!(data["draftKey"], "zayden:loadout-draft:1");
    assert_eq!(data["form"], form_value(&stored)?);
    assert_filled_markup(&html, &stored)?;

    let (status, html) =
        h.get("/admin/destiny2/loadouts/987654", Some(ADMIN_COOKIE)).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains(
        r#"<div class="page"><p class="error">Couldn't load the loadout: error running server function: loadout 987654 does not exist</p></div>"#
    ));

    pool.close().await;
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn endpoints_check_save_and_refuse_bad_emoji_names(
    options: PgPoolOptions,
    connect: PgConnectOptions,
) -> TestResult {
    let (h, pool) = harness(options, connect).await?;

    let blank = form_value(&web::admin::blank())?;
    let (status, check) = h
        .post_json(
            "/admin/destiny2/loadouts/editor/check",
            &blank,
            Some(ADMIN_COOKIE),
        )
        .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(check["ok"]["components"], 12);
    assert_eq!(check["ok"]["max_components"], 40);
    assert_eq!(check["ok"]["error"], "name is required");

    let (status, saved) = h
        .post_json(
            "/admin/destiny2/loadouts/editor/save",
            &blank,
            Some(ADMIN_COOKIE),
        )
        .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        saved["error"]
            .as_str()
            .is_some_and(|e| e.starts_with("error running server function: ")),
        "{saved}"
    );

    let mut edited = stored_form(&pool, 1).await?;
    edited.name = "Renamed by the editor".to_owned();
    edited.how_it_works = "\nStarts on line two".to_owned();
    let (status, saved) = h
        .post_json(
            "/admin/destiny2/loadouts/editor/save",
            &form_value(&edited)?,
            Some(ADMIN_COOKIE),
        )
        .await?;
    assert_eq!((status, saved), (StatusCode::OK, json!({ "ok": 1 })));
    assert_eq!(stored_form(&pool, 1).await?.name, "Renamed by the editor");
    let stored = stored_form(&pool, 1).await?;
    let (_, html) = h.get("/admin/destiny2/loadouts/1", Some(ADMIN_COOKIE)).await?;
    assert!(
        html.contains(&format!("rows=\"8\">\n{}</textarea>", stored.how_it_works))
    );
    assert_eq!(stored.how_it_works, "Starts on line two");

    let mut created = stored_form(&pool, 2).await?;
    created.id = None;
    created.name = "Brand new build".to_owned();
    let (status, saved) = h
        .post_json(
            "/admin/destiny2/loadouts/editor/save",
            &form_value(&created)?,
            Some(ADMIN_COOKIE),
        )
        .await?;
    assert_eq!(status, StatusCode::OK);
    let id = i32::try_from(saved["ok"].as_i64().ok_or("no id")?)?;
    assert!(id > 11);
    assert_eq!(stored_form(&pool, id).await?.name, "Brand new build");

    let emoji = json!({ "name": "Bad Name", "source": { "Url": "https://example.com/a.png" } });
    assert_eq!(
        h.post_json(
            "/admin/destiny2/loadouts/editor/emoji",
            &emoji,
            Some(ADMIN_COOKIE)
        )
        .await?,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "error": "emoji names are 2-32 lowercase letters, digits or underscores" })
        )
    );
    let reserved = json!({ "name": "auto_rifle", "source": { "Url": "https://example.com/a.png" } });
    assert_eq!(
        h.post_json(
            "/admin/destiny2/loadouts/editor/emoji",
            &reserved,
            Some(ADMIN_COOKIE)
        )
        .await?
        .1,
        json!({ "error": "`auto_rifle` is reserved for a built-in class, element, weapon or stat icon" })
    );
    let http = json!({ "name": "new_emoji", "source": { "Url": "http://example.com/a.png" } });
    assert_eq!(
        h.post_json(
            "/admin/destiny2/loadouts/editor/emoji",
            &http,
            Some(ADMIN_COOKIE)
        )
        .await?
        .1,
        json!({ "error": "the image link must be an https:// URL" })
    );

    let form = h
        .router
        .handle(
            Request::post("/admin/destiny2/loadouts/editor/check")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, ADMIN_COOKIE)
                .body(Body::from("name=x"))?,
        )
        .await;
    assert_eq!(form.status(), StatusCode::BAD_REQUEST);

    pool.close().await;
    Ok(())
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
