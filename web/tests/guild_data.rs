//! Guild loaders and saves against Postgres, driven through the app router so
//! each call authorizes from a real `session` cookie. Every case avoids
//! Discord: guild lists come from the seeded cache and no submitted id needs
//! an ownership check.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use honeypot::HoneypotConfig;
use http_body_util::BodyExt;
use jiff::{SignedDuration, Timestamp};
use sqlx::PgPool;
use ticket::{GuildId, HelperLinks, RoleId, SupportRoles};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::request::Request;
use topcoat::router::response::Response;
use topcoat::router::{Body, Router, StatusCode, header, path_param, route};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use web::auth::SessionUser;
use web::guild::dto::{
    AiSection,
    FamilySection,
    FaqSection,
    GeneralSection,
    HoneypotSection,
    LfgSection,
    MusicSection,
    PatreonStatus,
    SectionSettings,
    SupportSection,
    TempVoiceSection,
    Tier,
    YoutubeStatus,
};
use web::guild::{
    GuildError,
    GuildForm,
    command_permissions,
    faq,
    get_active_guild,
    get_section_settings,
    kofi,
    list_manageable_guilds,
    modules,
    patreon,
    settings,
    support,
    tier,
    youtube,
};
use web::state::{SessionUsersCache, UserGuildsCache, WebState};
use web::util::server_error_text;
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

type TestResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const ADMIN: &str = "session=admin-token";
const MEMBER: &str = "session=member-token";
const UPGRADE_URL: &str = "https://upgrade.example/pro";
const APP_ID: u64 = 123_456_789;

path_param!(section);
path_param!(kind);

fn render<T>(
    result: std::result::Result<T, GuildError>,
    ok: impl FnOnce(T) -> String,
) -> String {
    result.map_or_else(server_error_text, ok)
}

#[route(GET "/test/uncalled")]
async fn uncalled(cx: &Cx) -> Result<String> {
    let status =
        render(patreon::get_patreon_status(cx, "7").await, |s| format!("{s:?}"));
    let manage =
        render(patreon::can_manage_patreon(cx, "7").await, |m| m.to_string());
    let roles =
        render(support::list_support_roles(cx, "7").await, |r| format!("{r:?}"));
    let links =
        render(support::list_helper_links(cx, "7").await, |l| format!("{l:?}"));
    Ok(format!("{status}|{manage}|{roles}|{links}"))
}

#[route(GET "/test/server-tier-zero")]
async fn server_tier_zero(cx: &Cx) -> Result<String> {
    Ok(render(tier::guild_server_tier(cx, 0).await, |t| t.label().to_owned()))
}

#[route(GET "/test/guild-context")]
async fn command_context(cx: &Cx) -> Result<String> {
    Ok(render(command_permissions::guild_context(cx, "7").await, |ctx| {
        format!("{:?} {}", ctx.access, ctx.app_id)
    }))
}

#[route(GET "/test/guilds")]
async fn guilds(cx: &Cx) -> Result<String> {
    Ok(match list_manageable_guilds(cx).await {
        Ok(guilds) => guilds
            .iter()
            .map(|g| format!("{}:{}", g.id, g.name))
            .collect::<Vec<_>>()
            .join(","),
        Err(e) => server_error_text(e.redirect_unauthenticated()?),
    })
}

#[route(GET "/test/active")]
async fn active(cx: &Cx) -> Result<String> {
    Ok(render(get_active_guild(cx, "7").await, |g| {
        format!("{} {} {:?}", g.id, g.name, g.icon)
    }))
}

#[route(GET "/test/section/{section}")]
async fn section(cx: &Cx) -> Result<String> {
    let section: &str = path_param::<Section>(cx);
    Ok(render(get_section_settings(cx, "7", section).await, |s| format!("{s:?}")))
}

#[route(GET "/test/forbidden-section")]
async fn forbidden_section(cx: &Cx) -> Result<String> {
    Ok(render(get_section_settings(cx, "abc", "ai").await, |s| format!("{s:?}")))
}

#[route(GET "/test/modules")]
async fn module_cards(cx: &Cx) -> Result<String> {
    Ok(render(modules::list_guild_modules(cx, "7").await, |cards| {
        cards
            .iter()
            .map(|c| format!("{}={:?}", c.id, c.enabled))
            .collect::<Vec<_>>()
            .join(",")
    }))
}

#[route(GET "/test/tier")]
async fn user_tier(cx: &Cx) -> Result<String> {
    Ok(render(tier::get_user_tier(cx).await, |info| {
        format!("{:?} {:?}", info.tier.map(Tier::label), info.upgrade_url)
    }))
}

async fn save_form(
    cx: &Cx,
    kind: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    match kind {
        "channels" => {
            settings::save_channel_settings(
                cx,
                &settings::ChannelSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "roles" => {
            settings::save_role_settings(
                cx,
                &settings::RoleSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "ai" => {
            settings::save_ai_settings(
                cx,
                &settings::AiSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "family" => {
            settings::save_family_settings(
                cx,
                &settings::FamilySettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "honeypot" => {
            settings::save_honeypot_settings(
                cx,
                &settings::HoneypotSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "lfg" => {
            settings::save_lfg_settings(
                cx,
                &settings::LfgSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "music" => {
            settings::save_music_settings(
                cx,
                &settings::MusicSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "temp-voice" => {
            settings::save_temp_voice_settings(
                cx,
                &settings::TempVoiceSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "creator-channel" => {
            settings::create_temp_voice_creator_channel(
                cx,
                &settings::CreatorChannelForm::from_pairs(pairs)?,
            )
            .await
        },
        "support" => {
            support::save_support_settings(
                cx,
                &support::SupportSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "idle" => {
            support::save_idle_settings(
                cx,
                &support::IdleSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "stale" => {
            support::save_stale_settings(
                cx,
                &support::StaleSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "suggestions" => {
            support::save_suggestions_settings(
                cx,
                &support::SuggestionsSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "add-support-role" => {
            support::add_support_role(
                cx,
                &support::SupportRoleForm::from_pairs(pairs)?,
            )
            .await
        },
        "remove-support-role" => {
            support::remove_support_role(
                cx,
                &support::SupportRoleForm::from_pairs(pairs)?,
            )
            .await
        },
        "add-helper-link" => {
            support::add_helper_link(
                cx,
                &support::AddHelperLinkForm::from_pairs(pairs)?,
            )
            .await
        },
        "remove-helper-link" => {
            support::remove_helper_link(
                cx,
                &support::RemoveHelperLinkForm::from_pairs(pairs)?,
            )
            .await
        },
        "faq" => {
            faq::save_faq_settings(cx, &faq::FaqSettingsForm::from_pairs(pairs)?)
                .await
        },
        "faq-key" => {
            faq::save_faq_wiki_key(cx, &faq::FaqWikiKeyForm::from_pairs(pairs)?)
                .await
        },
        "faq-tuning" => {
            faq::save_faq_tuning(cx, &faq::FaqTuningForm::from_pairs(pairs)?).await
        },
        "patreon" => {
            patreon::save_patreon_settings(
                cx,
                &patreon::PatreonSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "patreon-disconnect" => {
            patreon::disconnect_patreon(cx, &GuildForm::from_pairs(pairs)?).await
        },
        "youtube" => {
            youtube::save_youtube_settings(
                cx,
                &youtube::YoutubeSettingsForm::from_pairs(pairs)?,
            )
            .await
        },
        "youtube-disconnect" => {
            youtube::disconnect_youtube(cx, &GuildForm::from_pairs(pairs)?).await
        },
        "kofi" => {
            kofi::link_kofi_email(cx, &kofi::KofiEmailForm::from_pairs(pairs)?).await
        },
        "module" => {
            let form = modules::ModuleToggleForm::from_pairs(pairs)?;
            let enabled = form.enabled()?;
            modules::set_module_enabled(cx, &form.guild, &form.module_id, enabled)
                .await
        },
        other => Err(GuildError::UnknownField(other.to_owned())),
    }
}

#[route(POST "/test/save/{kind}")]
async fn save(cx: &Cx, Form(pairs): Form<Vec<(String, String)>>) -> Result<String> {
    let kind: &str = path_param::<Kind>(cx);
    Ok(render(save_form(cx, kind, pairs).await, |()| "ok".to_owned()))
}

fn config(zayden_id: u64) -> BotConfig {
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
        zayden_id,
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
        upgrade_url: Some(UPGRADE_URL.to_owned()),
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
                guild(8, "Guild 8", Permissions::SEND_MESSAGES),
            ]),
        )
        .await;
    cache
        .insert(43, Arc::from([guild(7, "Guild 7", Permissions::SEND_MESSAGES)]))
        .await;
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

async fn harness(pool: PgPool) -> TestResult<Harness> {
    harness_for(pool, APP_ID).await
}

async fn harness_for(pool: PgPool, zayden_id: u64) -> TestResult<Harness> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let config = config(zayden_id);
    let app = Arc::new(ZaydenAppState::new(pool.clone(), &config));
    let state = WebState::new(app, &config)?;
    seed_guilds(&state.discord.user_guilds).await;
    seed_users(&state.discord.users, &[41, 43]).await;
    insert_session(&pool, "admin-token", 41).await?;
    insert_session(&pool, "member-token", 43).await?;

    let base = Router::builder()
        .app_context(state)
        .route(guilds)
        .route(active)
        .route(section)
        .route(forbidden_section)
        .route(module_cards)
        .route(user_tier)
        .route(save)
        .route(uncalled)
        .route(server_tier_zero)
        .route(command_context);

    Ok(Harness { router: web::router(base), pool })
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

    async fn section(&self, name: &str) -> TestResult<String> {
        self.text(&format!("/test/section/{name}"), Some(ADMIN)).await
    }

    async fn save(
        &self,
        kind: &str,
        body: &str,
        cookie: Option<&str>,
    ) -> TestResult<String> {
        let mut request = Request::post(format!("/test/save/{kind}"))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let response =
            self.router.handle(request.body(Body::from(body.to_owned()))?).await;
        body_text(response).await
    }

    async fn admin_save(&self, kind: &str, body: &str) -> TestResult<String> {
        self.save(kind, body, Some(ADMIN)).await
    }
}

async fn body_text(response: Response) -> TestResult<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn shown(message: &str) -> String {
    server_error_text(message)
}

fn debug(settings: &SectionSettings) -> String {
    format!("{settings:?}")
}

#[sqlx::test(migrations = "../migrations")]
async fn the_guild_list_holds_only_guilds_the_user_manages(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(app.text("/test/guilds", Some(ADMIN)).await.unwrap(), "7:Guild 7");
    assert_eq!(app.text("/test/guilds", Some(MEMBER)).await.unwrap(), "");
}

#[sqlx::test(migrations = "../migrations")]
async fn a_signed_out_guild_list_redirects_to_login(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let response = app.get("/test/guilds", None).await.unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/login");
}

#[sqlx::test(migrations = "../migrations")]
async fn the_active_guild_comes_from_the_users_own_list(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.text("/test/active", Some(ADMIN)).await.unwrap(),
        "7 Guild 7 None"
    );
    assert_eq!(
        app.text("/test/active", None).await.unwrap(),
        shown("unauthenticated")
    );
    assert_eq!(
        app.text("/test/active", Some(MEMBER)).await.unwrap(),
        shown("forbidden")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn section_loads_authorize_first(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.text("/test/section/ai", None).await.unwrap(),
        shown("unauthenticated")
    );
    assert_eq!(
        app.text("/test/section/ai", Some(MEMBER)).await.unwrap(),
        shown("forbidden")
    );
    assert_eq!(
        app.text("/test/forbidden-section", Some(ADMIN)).await.unwrap(),
        shown("invalid guild id")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn an_unknown_section_reads_the_general_one(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let general = debug(&SectionSettings::General(GeneralSection {
        rules_channel_id: None,
        general_channel_id: None,
        spoiler_channel_id: None,
        artist_role_id: None,
        sleep_role_id: None,
        verified_role_id: None,
    }));

    assert_eq!(app.section("general").await.unwrap(), general);
    assert_eq!(app.section("nope").await.unwrap(), general);
}

#[sqlx::test(migrations = "../migrations")]
async fn blank_ids_clear_settings_without_asking_discord(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let saves = [
        (
            "channels",
            "guild=7&rules_channel_id=&general_channel_id=&spoiler_channel_id=",
        ),
        ("roles", "guild=7&artist_role_id=&sleep_role_id=&verified_role_id="),
        ("temp-voice", "guild=7&temp_voice_category=&temp_voice_creator_channel="),
        ("lfg", "guild=7&lfg_channel_id=&lfg_role_id=&lfg_scheduled_thread_id="),
    ];
    for (kind, body) in saves {
        assert_eq!(app.admin_save(kind, body).await.unwrap(), "ok", "{kind}");
    }

    assert_eq!(
        app.section("temp-voice").await.unwrap(),
        debug(&SectionSettings::TempVoice(TempVoiceSection {
            category: None,
            creator_channel: None,
        }))
    );
    assert_eq!(
        app.section("lfg").await.unwrap(),
        debug(&SectionSettings::Lfg(LfgSection {
            channel_id: None,
            role_id: None,
            scheduled_thread_id: None,
        }))
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn ai_settings_round_trip(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.admin_save("ai", "guild=7&enabled=true&channel_id=").await.unwrap(),
        "ok"
    );

    assert_eq!(
        app.section("ai").await.unwrap(),
        debug(&SectionSettings::Ai(AiSection { enabled: true, channel_id: None }))
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn family_partners_are_at_least_one(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.admin_save("family", "guild=7&max_partners=3").await.unwrap(),
        "ok"
    );
    assert_eq!(
        app.section("family").await.unwrap(),
        debug(&SectionSettings::Family(FamilySection {
            max_partners: "3".to_owned()
        }))
    );

    assert_eq!(
        app.admin_save("family", "guild=7&max_partners=0").await.unwrap(),
        "ok"
    );
    assert_eq!(
        app.section("family").await.unwrap(),
        debug(&SectionSettings::Family(FamilySection {
            max_partners: "1".to_owned()
        }))
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn music_clamps_the_disconnect_delay(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let body = "guild=7&dj_role_id=&auto_disconnect_secs=9999&announce_now_playing=true&announce_channel_id=";
    assert_eq!(app.admin_save("music", body).await.unwrap(), "ok");

    assert_eq!(
        app.section("music").await.unwrap(),
        debug(&SectionSettings::Music(MusicSection {
            dj_role_id: None,
            auto_disconnect_secs: "600".to_owned(),
            announce_now_playing: true,
            announce_channel_id: None,
        }))
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn honeypot_validates_through_its_own_form_rules(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let body = "guild=7&channel_id=&exempt_admins=true&exempt_role_id=&purge_seconds=9999999";
    assert_eq!(app.admin_save("honeypot", body).await.unwrap(), "ok");
    assert_eq!(
        app.section("honeypot").await.unwrap(),
        debug(&SectionSettings::Honeypot(HoneypotSection {
            channel_id: None,
            exempt_admins: true,
            exempt_role_id: None,
            purge_seconds: "604800".to_owned(),
        }))
    );

    let refused = HoneypotConfig::from_form("abc", false, "", "").unwrap_err();
    let body =
        "guild=7&channel_id=abc&exempt_admins=false&exempt_role_id=&purge_seconds=";
    assert_eq!(
        app.admin_save("honeypot", body).await.unwrap(),
        shown(&refused.to_string())
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_creator_channel_needs_a_category(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.admin_save("creator-channel", "guild=7&temp_voice_category=")
            .await
            .unwrap(),
        shown("select a category first")
    );
    assert_eq!(
        app.save("creator-channel", "guild=7&temp_voice_category=", Some(MEMBER))
            .await
            .unwrap(),
        shown("forbidden")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn support_settings_round_trip(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let saves = [
        (
            "support",
            "guild=7&support_channel_id=&solved_tag_id=5&closed_tag_id=x&solved_archive_secs=-3",
        ),
        (
            "idle",
            "guild=7&idle_enabled=true&idle_after_secs=10&idle_close_enabled=false&idle_close_after_secs=",
        ),
        (
            "stale",
            "guild=7&stale_enabled=true&stale_tag_id=6&stale_after_secs=99999999",
        ),
        (
            "suggestions",
            "guild=7&suggestions_channel_id=&review_channel_id=&promote_threshold=10&demote_threshold=12",
        ),
        (
            "faq",
            "guild=7&enabled=true&auto_triage=false&auto_generate=true&wiki_url=https://wiki.example.com/&wiki_locale=",
        ),
        ("faq-key", "guild=7&wiki_api_key=secret&keep_wiki_api_key=false"),
        (
            "faq-tuning",
            "guild=7&max_results=99&answer_max_tokens=1&answer_temperature=3",
        ),
    ];
    for (kind, body) in saves {
        assert_eq!(app.admin_save(kind, body).await.unwrap(), "ok", "{kind}");
    }

    let expected = SupportSection {
        support_channel_id: None,
        solved_tag_id: Some("5".to_owned()),
        closed_tag_id: None,
        solved_archive_secs: "-1".to_owned(),
        idle_enabled: true,
        idle_after_secs: "3600".to_owned(),
        idle_close_enabled: false,
        idle_close_after_secs: "86400".to_owned(),
        stale_enabled: true,
        stale_tag_id: Some("6".to_owned()),
        stale_after_secs: "2592000".to_owned(),
        suggestions_channel_id: None,
        review_channel_id: None,
        promote_threshold: "10".to_owned(),
        demote_threshold: "9".to_owned(),
        faq: FaqSection {
            enabled: true,
            auto_triage: false,
            auto_generate: true,
            wiki_url: "https://wiki.example.com".to_owned(),
            wiki_api_key_set: true,
            wiki_locale: "en".to_owned(),
            max_results: "25".to_owned(),
            answer_max_tokens: "64".to_owned(),
            answer_temperature: "2".to_owned(),
        },
        support_roles: Ok(Vec::new()),
        helper_links: Ok(Vec::new()),
    };

    assert_eq!(
        app.section("support").await.unwrap(),
        debug(&SectionSettings::Support(Box::new(expected)))
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_blank_wiki_key_with_keep_leaves_the_stored_key(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let key_set = |text: &str| text.contains("wiki_api_key_set: true");

    app.admin_save("faq-key", "guild=7&wiki_api_key=secret&keep_wiki_api_key=false")
        .await
        .unwrap();
    assert_eq!(
        app.admin_save("faq-key", "guild=7&wiki_api_key=&keep_wiki_api_key=true")
            .await
            .unwrap(),
        "ok"
    );
    assert!(key_set(&app.section("support").await.unwrap()));

    app.admin_save("faq-key", "guild=7&wiki_api_key=&keep_wiki_api_key=false")
        .await
        .unwrap();
    assert!(!key_set(&app.section("support").await.unwrap()));
}

#[sqlx::test(migrations = "../migrations")]
async fn the_wiki_url_is_validated_before_saving(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let body = "guild=7&enabled=true&auto_triage=true&auto_generate=true&wiki_url=ftp://wiki&wiki_locale=en";
    assert_eq!(
        app.admin_save("faq", body).await.unwrap(),
        shown("the wiki URL must start with http:// or https://")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn support_roles_are_listed_and_removed_without_an_ownership_check(
    pool: PgPool,
) {
    let app = harness(pool).await.unwrap();
    SupportRoles::add(&app.pool, GuildId::new(7), RoleId::new(5)).await.unwrap();

    assert!(
        app.section("support")
            .await
            .unwrap()
            .contains(r#"support_roles: Ok(["5"])"#)
    );

    assert_eq!(
        app.admin_save("add-support-role", "guild=7&role_id=abc").await.unwrap(),
        shown("invalid role")
    );
    assert_eq!(
        app.admin_save("remove-support-role", "guild=7&role_id=5").await.unwrap(),
        "ok"
    );
    assert_eq!(
        SupportRoles::ids(&app.pool, GuildId::new(7)).await.unwrap(),
        Vec::<RoleId>::new()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn helper_links_are_validated_then_stored_normalized(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.admin_save(
            "add-helper-link",
            "guild=7&user_id=x&link=https://a.example"
        )
        .await
        .unwrap(),
        shown("invalid user id")
    );
    assert_eq!(
        app.admin_save("add-helper-link", "guild=7&user_id=9&link=ftp://a.example")
            .await
            .unwrap(),
        shown("link must be an http:// or https:// address")
    );
    assert_eq!(
        app.admin_save(
            "add-helper-link",
            "guild=7&user_id=9&link=https://a.example"
        )
        .await
        .unwrap(),
        "ok"
    );

    let links = HelperLinks::list(&app.pool, GuildId::new(7)).await.unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].user_id.get(), 9);
    assert_eq!(links[0].link, "https://a.example/");

    assert_eq!(
        app.admin_save("remove-helper-link", "guild=7&user_id=9").await.unwrap(),
        "ok"
    );
    assert!(HelperLinks::list(&app.pool, GuildId::new(7)).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn patreon_and_youtube_start_disconnected(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.section("patreon").await.unwrap(),
        debug(&SectionSettings::Patreon(Ok(PatreonStatus::default())))
    );
    assert_eq!(
        app.section("youtube").await.unwrap(),
        debug(&SectionSettings::Youtube(Ok(YoutubeStatus::default())))
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn announcement_channels_clear_or_must_parse(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    for kind in ["patreon", "youtube"] {
        let public_only = if kind == "patreon" { "&public_only=true" } else { "" };

        assert_eq!(
            app.admin_save(kind, &format!("guild=7&channel_id={public_only}"))
                .await
                .unwrap(),
            "ok",
            "{kind}"
        );
        assert_eq!(
            app.admin_save(kind, &format!("guild=7&channel_id=abc{public_only}"))
                .await
                .unwrap(),
            shown("invalid channel id"),
            "{kind}"
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn disconnecting_nothing_succeeds(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(app.admin_save("patreon-disconnect", "guild=7").await.unwrap(), "ok");
    assert_eq!(app.admin_save("youtube-disconnect", "guild=7").await.unwrap(), "ok");
    assert_eq!(
        app.save("youtube-disconnect", "guild=7", Some(MEMBER)).await.unwrap(),
        shown("forbidden")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_kofi_email_links_once(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.save("kofi", "email=nope", None).await.unwrap(),
        shown("invalid email")
    );
    assert_eq!(
        app.save("kofi", "email=me@example.com", None).await.unwrap(),
        shown("unauthenticated")
    );
    assert_eq!(
        app.save("kofi", "email=%20Me@Example.com%20", Some(ADMIN)).await.unwrap(),
        "ok"
    );
    assert_eq!(
        app.save("kofi", "email=me@example.com", Some(MEMBER)).await.unwrap(),
        shown("This Ko-fi email is already linked to an account.")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn module_cards_read_stored_states_and_settings(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let cards = app.text("/test/modules", Some(ADMIN)).await.unwrap();
    assert!(cards.starts_with("music=None,"), "{cards}");
    assert!(cards.contains("patreon=Some(false)"), "{cards}");
    assert!(cards.contains("youtube=Some(false)"), "{cards}");
    assert!(cards.contains("ai=Some(false)"), "{cards}");

    assert_eq!(
        app.admin_save("module", "guild=7&module_id=music&enabled=true")
            .await
            .unwrap(),
        "ok"
    );
    assert_eq!(
        app.admin_save("module", "guild=7&module_id=ai&enabled=true").await.unwrap(),
        "ok"
    );

    let cards = app.text("/test/modules", Some(ADMIN)).await.unwrap();
    assert!(cards.starts_with("music=Some(true),"), "{cards}");
    assert!(cards.contains("ai=Some(true)"), "{cards}");
}

#[sqlx::test(migrations = "../migrations")]
async fn module_toggles_refuse_what_they_cannot_switch(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.save("module", "guild=7&module_id=nope&enabled=true", None)
            .await
            .unwrap(),
        shown("unknown module")
    );
    assert_eq!(
        app.admin_save("module", "guild=7&module_id=patreon&enabled=true")
            .await
            .unwrap(),
        shown(
            "Patreon is switched on from its own settings page, not from this toggle."
        )
    );
    assert_eq!(
        app.save("module", "guild=7&module_id=music&enabled=true", Some(MEMBER))
            .await
            .unwrap(),
        shown("forbidden")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn the_user_tier_is_free_until_entitled(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    let url = format!("{:?}", Some(UPGRADE_URL));
    assert_eq!(app.text("/test/tier", None).await.unwrap(), format!("None {url}"));
    assert_eq!(
        app.text("/test/tier", Some(ADMIN)).await.unwrap(),
        format!("{:?} {url}", Some("Free"))
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn connection_failures_are_shown_with_one_prefix(pool: PgPool) {
    let app = harness(pool).await.unwrap();
    app.section("ai").await.unwrap();
    app.pool.close().await;

    for name in ["patreon", "youtube"] {
        let text = app.section(name).await.unwrap();
        assert!(text.contains("(Err(\"error running server function: "), "{text}");
        assert_eq!(
            text.matches("error running server function: ").count(),
            1,
            "{text}"
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn uncalled_guild_reads_authorize_first(pool: PgPool) {
    let app = harness(pool).await.unwrap();
    SupportRoles::add(&app.pool, GuildId::new(7), RoleId::new(5)).await.unwrap();

    assert_eq!(
        app.text("/test/uncalled", Some(ADMIN)).await.unwrap(),
        format!("{:?}|true|[\"5\"]|[]", PatreonStatus::default())
    );

    let forbidden = shown("forbidden");
    assert_eq!(
        app.text("/test/uncalled", Some(MEMBER)).await.unwrap(),
        [forbidden.as_str(); 4].join("|")
    );

    let unauthenticated = shown("unauthenticated");
    assert_eq!(
        app.text("/test/uncalled", None).await.unwrap(),
        [unauthenticated.as_str(); 4].join("|")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_zero_guild_id_has_no_server_tier(pool: PgPool) {
    let app = harness(pool).await.unwrap();

    assert_eq!(
        app.text("/test/server-tier-zero", None).await.unwrap(),
        shown("invalid guild id")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn command_permissions_need_an_application_id(pool: PgPool) {
    let app = harness(pool.clone()).await.unwrap();
    assert_eq!(
        app.text("/test/guild-context", Some(ADMIN)).await.unwrap(),
        format!("Member {APP_ID}")
    );
    assert_eq!(
        app.text("/test/guild-context", Some(MEMBER)).await.unwrap(),
        shown("forbidden")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_zero_application_id_is_refused(pool: PgPool) {
    let app = harness_for(pool, 0).await.unwrap();

    assert_eq!(
        app.text("/test/guild-context", Some(ADMIN)).await.unwrap(),
        shown("invalid application id")
    );
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
