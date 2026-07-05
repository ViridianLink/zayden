#![cfg_attr(
    not(feature = "ssr"),
    expect(
        clippy::unused_async_trait_impl,
        reason = "Leptos #[server] macro generates an unawaited `run_body` stub for non-ssr builds"
    )
)]

use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::{Stylesheet, Title, provide_meta_context};
use leptos_router::components::{
    A,
    Outlet,
    ParentRoute,
    Redirect,
    Route,
    Router,
    Routes,
};
use leptos_router::hooks::use_params_map;
use leptos_router::path;
use serde::{Deserialize, Serialize};

/// Context type carrying the optional Pro upgrade URL provided by the server.
///
/// Provided via `leptos_routes_with_context` so server functions can include it
/// in their responses without additional round-trips.
#[derive(Clone)]
pub struct UpgradeUrl(pub Option<String>);

#[derive(Clone, Serialize, Deserialize)]
struct UserTierInfo {
    /// Tier key: "free", "pro", or "enterprise". Empty when unauthenticated.
    tier: String,
    /// Pro upgrade URL forwarded from `UpgradeUrl` context, if configured.
    upgrade_url: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct GuildInfo {
    id: String,
    name: String,
    icon: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct GuildSettings {
    support_channel_id: Option<String>,
    support_role_id: Option<String>,
    faq_channel_id: Option<String>,
    suggestions_channel_id: Option<String>,
    review_channel_id: Option<String>,
    rules_channel_id: Option<String>,
    general_channel_id: Option<String>,
    spoiler_channel_id: Option<String>,
    artist_role_id: Option<String>,
    sleep_role_id: Option<String>,
    temp_voice_category: Option<String>,
    temp_voice_creator_channel: Option<String>,
    lfg_channel_id: Option<String>,
    lfg_role_id: Option<String>,
    lfg_scheduled_thread_id: Option<String>,
    is_pro: bool,
}

/// Server function that retrieves the list of Discord guilds the current user
/// can manage (has `MANAGE_GUILD` or `ADMINISTRATOR` permission bits).
///
/// Returns a redirect to `/login` on the server side when the session is
/// missing or expired, and propagates the error to the caller so the component
/// can display a sensible message on the client side.
#[server]
async fn list_manageable_guilds() -> Result<Vec<GuildInfo>, ServerFnError> {
    use leptos_axum::extract;
    use reqwest::Client;
    use sqlx::{PgPool, Row};
    use tower_cookies::Cookies;

    #[derive(serde::Deserialize)]
    struct DiscordGuild {
        id: String,
        name: String,
        icon: Option<String>,
        permissions: String,
    }

    const MANAGE_GUILD: u64 = 0x20;
    const ADMINISTRATOR: u64 = 0x08;

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };
    let Some(http) = use_context::<Client>() else {
        return Err(ServerFnError::ServerError("missing HTTP client".to_string()));
    };

    let cookies: Cookies = match extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        leptos_axum::redirect("/login");
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let access_token: String = match sqlx::query(
        "SELECT discord_access_token FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(r)) => r.get("discord_access_token"),
        Ok(None) => {
            leptos_axum::redirect("/login");
            return Err(ServerFnError::ServerError("unauthenticated".to_string()));
        },
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let resp = match http
        .get("https://discord.com/api/v10/users/@me/guilds")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    if !resp.status().is_success() {
        return Err(ServerFnError::ServerError(format!(
            "Discord API returned {}",
            resp.status()
        )));
    }

    let all_guilds: Vec<DiscordGuild> = match resp.json().await {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let guilds = all_guilds
        .into_iter()
        .filter(|g| {
            g.permissions
                .parse::<u64>()
                .is_ok_and(|p| p & ADMINISTRATOR != 0 || p & MANAGE_GUILD != 0)
        })
        .map(|g| GuildInfo { id: g.id, name: g.name, icon: g.icon })
        .collect();

    Ok(guilds)
}

#[server]
async fn get_guild_settings(
    guild_id: String,
) -> Result<GuildSettings, ServerFnError> {
    use std::sync::Arc;

    use leptos_axum::extract;
    use reqwest::Client;
    use sqlx::PgPool;
    use tower_cookies::Cookies;
    use zayden_app::entitlement::types::{EntitlementScope, Tier};
    use zayden_app::state::AppState;

    #[derive(serde::Deserialize)]
    struct DiscordGuild {
        id: String,
        permissions: String,
    }

    const MANAGE_GUILD: u64 = 0x20;
    const ADMINISTRATOR: u64 = 0x08;

    let Ok(guild_id_i64) = guild_id.parse::<i64>() else {
        return Err(ServerFnError::ServerError("invalid guild id".to_string()));
    };
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };
    let Some(http) = use_context::<Client>() else {
        return Err(ServerFnError::ServerError("missing HTTP client".to_string()));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    let cookies: Cookies = match extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        leptos_axum::redirect("/login");
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let row = match sqlx::query(
        "SELECT discord_access_token, discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let Some(row) = row else {
        leptos_axum::redirect("/login");
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };
    use sqlx::Row as _;
    let access_token: String = row.get("discord_access_token");
    let discord_user_id: i64 = row.get("discord_user_id");
    let discord_user_id_u64 = discord_user_id.cast_unsigned();

    let guilds_resp = match http
        .get("https://discord.com/api/v10/users/@me/guilds")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    if !guilds_resp.status().is_success() {
        return Err(ServerFnError::ServerError(
            "failed to fetch guild list from Discord".to_string(),
        ));
    }
    let all_guilds: Vec<DiscordGuild> = match guilds_resp.json().await {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let has_access = all_guilds.iter().any(|g| {
        g.id == guild_id
            && g.permissions
                .parse::<u64>()
                .is_ok_and(|p| p & ADMINISTRATOR != 0 || p & MANAGE_GUILD != 0)
    });
    if !has_access {
        return Err(ServerFnError::ServerError("forbidden".to_string()));
    }

    fn opt_str(v: Option<i64>) -> Option<String> {
        v.map(|n| n.to_string())
    }
    fn app_err(e: sqlx::Error) -> ServerFnError {
        ServerFnError::ServerError(e.to_string())
    }

    let support = app.settings.support.get(guild_id_i64).await.map_err(app_err)?;
    let suggestions =
        app.settings.suggestions.get(guild_id_i64).await.map_err(app_err)?;
    let channels = app.settings.channels.get(guild_id_i64).await.map_err(app_err)?;
    let roles = app.settings.roles.get(guild_id_i64).await.map_err(app_err)?;
    let temp_voice =
        app.settings.temp_voice.get(guild_id_i64).await.map_err(app_err)?;
    let lfg = app.settings.lfg.get(guild_id_i64).await.map_err(app_err)?;

    let scope = EntitlementScope::UserInGuild(discord_user_id_u64, guild_id_u64);
    let is_pro = app.entitlements.allows(scope, Tier::Pro).await;

    Ok(GuildSettings {
        support_channel_id: opt_str(support.support_channel_id),
        support_role_id: opt_str(support.support_role_id),
        faq_channel_id: opt_str(support.faq_channel_id),
        suggestions_channel_id: opt_str(suggestions.suggestions_channel_id),
        review_channel_id: opt_str(suggestions.review_channel_id),
        rules_channel_id: opt_str(channels.rules_channel_id),
        general_channel_id: opt_str(channels.general_channel_id),
        spoiler_channel_id: opt_str(channels.spoiler_channel_id),
        artist_role_id: opt_str(roles.artist_role_id),
        sleep_role_id: opt_str(roles.sleep_role_id),
        temp_voice_category: opt_str(temp_voice.temp_voice_category),
        temp_voice_creator_channel: opt_str(temp_voice.temp_voice_creator_channel),
        lfg_channel_id: opt_str(lfg.lfg_channel_id),
        lfg_role_id: opt_str(lfg.lfg_role_id),
        lfg_scheduled_thread_id: opt_str(lfg.lfg_scheduled_thread_id),
        is_pro,
    })
}

/// SSR-only guard shared by all per-fieldset save server functions.
///
/// Validates the session cookie, confirms the caller holds `MANAGE_GUILD` on
/// `guild_id_str`, and checks that a Pro entitlement covers the
/// `UserInGuild(user, guild)` scope.  Returns `(guild_id_i64, user_id)` on
/// success.
///
/// Returns `Err(ServerFnError::ServerError("pro_required"))` when the tier
/// check fails, so the component can render a targeted upgrade prompt.
#[cfg(feature = "ssr")]
async fn guild_write_guard(guild_id_str: &str) -> Result<(i64, u64), ServerFnError> {
    use std::sync::Arc;

    use leptos_axum::extract;
    use reqwest::Client;
    use sqlx::PgPool;
    use tower_cookies::Cookies;
    use zayden_app::entitlement::types::{EntitlementScope, Tier};
    use zayden_app::state::AppState;

    #[derive(serde::Deserialize)]
    struct DiscordGuild {
        id: String,
        permissions: String,
    }

    const MANAGE_GUILD: u64 = 0x20;
    const ADMINISTRATOR: u64 = 0x08;

    let Ok(guild_id_i64) = guild_id_str.parse::<i64>() else {
        return Err(ServerFnError::ServerError("invalid guild id".to_string()));
    };
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };
    let Some(http) = use_context::<Client>() else {
        return Err(ServerFnError::ServerError("missing HTTP client".to_string()));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    let cookies: Cookies = match extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let row = match sqlx::query(
        "SELECT discord_access_token, discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let Some(row) = row else {
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };
    use sqlx::Row as _;
    let access_token: String = row.get("discord_access_token");
    let discord_user_id: i64 = row.get("discord_user_id");
    let discord_user_id_u64 = discord_user_id.cast_unsigned();

    let guilds_resp = match http
        .get("https://discord.com/api/v10/users/@me/guilds")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    if !guilds_resp.status().is_success() {
        return Err(ServerFnError::ServerError(
            "failed to fetch guild list from Discord".to_string(),
        ));
    }
    let all_guilds: Vec<DiscordGuild> = match guilds_resp.json().await {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    let has_access = all_guilds.iter().any(|g| {
        g.id == guild_id_str
            && g.permissions
                .parse::<u64>()
                .is_ok_and(|p| p & ADMINISTRATOR != 0 || p & MANAGE_GUILD != 0)
    });
    if !has_access {
        return Err(ServerFnError::ServerError("forbidden".to_string()));
    }

    let scope = EntitlementScope::UserInGuild(discord_user_id_u64, guild_id_u64);
    if !app.entitlements.allows(scope, Tier::Pro).await {
        return Err(ServerFnError::ServerError("pro_required".to_string()));
    }

    Ok((guild_id_i64, discord_user_id_u64))
}

/// Persist support-section settings (support channel, role, FAQ, suggestions,
/// review).  Each fieldset has its own save action to stay within the 7-argument
/// lint limit while keeping all auth/MANAGE_GUILD/Pro validation in one place.
#[server]
async fn save_support_settings(
    guild: String,
    support_channel_id: String,
    support_role_id: String,
    faq_channel_id: String,
    suggestions_channel_id: String,
    review_channel_id: String,
) -> Result<(), ServerFnError> {
    use std::sync::Arc;

    use zayden_app::state::AppState;

    fn parse_id(s: &str) -> Option<i64> {
        let t = s.trim();
        if t.is_empty() { None } else { t.parse().ok() }
    }
    fn app_err(e: sqlx::Error) -> ServerFnError {
        ServerFnError::ServerError(e.to_string())
    }

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .support
        .update(guild_id_i64, |p| {
            p.support_channel_id = parse_id(&support_channel_id);
            p.support_role_id = parse_id(&support_role_id);
            p.faq_channel_id = parse_id(&faq_channel_id);
        })
        .await
        .map_err(app_err)?;

    app.settings
        .suggestions
        .update(guild_id_i64, |p| {
            p.suggestions_channel_id = parse_id(&suggestions_channel_id);
            p.review_channel_id = parse_id(&review_channel_id);
        })
        .await
        .map(|_| ())
        .map_err(app_err)
}

/// Persist channel-section settings (rules, general, spoiler).
#[server]
async fn save_channel_settings(
    guild: String,
    rules_channel_id: String,
    general_channel_id: String,
    spoiler_channel_id: String,
) -> Result<(), ServerFnError> {
    use std::sync::Arc;

    use zayden_app::state::AppState;

    fn parse_id(s: &str) -> Option<i64> {
        let t = s.trim();
        if t.is_empty() { None } else { t.parse().ok() }
    }

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .channels
        .update(guild_id_i64, |p| {
            p.rules_channel_id = parse_id(&rules_channel_id);
            p.general_channel_id = parse_id(&general_channel_id);
            p.spoiler_channel_id = parse_id(&spoiler_channel_id);
        })
        .await
        .map(|_| ())
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

/// Persist role-section settings (artist, sleep).
#[server]
async fn save_role_settings(
    guild: String,
    artist_role_id: String,
    sleep_role_id: String,
) -> Result<(), ServerFnError> {
    use std::sync::Arc;

    use zayden_app::state::AppState;

    fn parse_id(s: &str) -> Option<i64> {
        let t = s.trim();
        if t.is_empty() { None } else { t.parse().ok() }
    }

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .roles
        .update(guild_id_i64, |p| {
            p.artist_role_id = parse_id(&artist_role_id);
            p.sleep_role_id = parse_id(&sleep_role_id);
        })
        .await
        .map(|_| ())
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

/// Persist temp-voice-section settings (category, creator channel).
#[server]
async fn save_temp_voice_settings(
    guild: String,
    temp_voice_category: String,
    temp_voice_creator_channel: String,
) -> Result<(), ServerFnError> {
    use std::sync::Arc;

    use zayden_app::state::AppState;

    fn parse_id(s: &str) -> Option<i64> {
        let t = s.trim();
        if t.is_empty() { None } else { t.parse().ok() }
    }

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .temp_voice
        .update(guild_id_i64, |p| {
            p.temp_voice_category = parse_id(&temp_voice_category);
            p.temp_voice_creator_channel = parse_id(&temp_voice_creator_channel);
        })
        .await
        .map(|_| ())
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

/// Persist LFG-section settings (channel, role, scheduled thread).
#[server]
async fn save_lfg_settings(
    guild: String,
    lfg_channel_id: String,
    lfg_role_id: String,
    lfg_scheduled_thread_id: String,
) -> Result<(), ServerFnError> {
    use std::sync::Arc;

    use zayden_app::state::AppState;

    fn parse_id(s: &str) -> Option<i64> {
        let t = s.trim();
        if t.is_empty() { None } else { t.parse().ok() }
    }

    let (guild_id_i64, _) = guild_write_guard(&guild).await?;

    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };

    app.settings
        .lfg
        .update(guild_id_i64, |p| {
            p.lfg_channel_id = parse_id(&lfg_channel_id);
            p.lfg_role_id = parse_id(&lfg_role_id);
            p.lfg_scheduled_thread_id = parse_id(&lfg_scheduled_thread_id);
        })
        .await
        .map(|_| ())
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

/// Returns the active billing tier for the current user plus the configured
/// upgrade URL (if any).
///
/// Returns an empty `tier` string when the request is unauthenticated rather
/// than redirecting, so the NavBar tier badge can render nothing without
/// interfering with pages that do their own session handling.
#[server]
async fn get_user_tier() -> Result<UserTierInfo, ServerFnError> {
    use std::sync::Arc;

    use leptos_axum::extract;
    use sqlx::{PgPool, Row};
    use tower_cookies::Cookies;
    use zayden_app::state::AppState;

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };
    let upgrade_url = use_context::<UpgradeUrl>().and_then(|u| u.0);

    let cookies: Cookies = match extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Ok(UserTierInfo { tier: String::new(), upgrade_url });
    };

    let row = match sqlx::query(
        "SELECT discord_user_id FROM web_sessions WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let Some(row) = row else {
        return Ok(UserTierInfo { tier: String::new(), upgrade_url });
    };
    let user_id = row.get::<i64, _>("discord_user_id").cast_unsigned();

    let tier = app.entitlements.user_tier(user_id).await;
    Ok(UserTierInfo { tier: tier.as_str().to_owned(), upgrade_url })
}

/// Server function that checks whether the current request carries a valid
/// `session` cookie.  On the server, if the session is valid, it also calls
/// `leptos_axum::redirect("/guilds")` so that the HTTP response becomes a 302
/// before any page HTML is sent to the client.
#[server]
async fn check_session() -> Result<bool, ServerFnError> {
    use leptos_axum::extract;
    use sqlx::PgPool;
    use tower_cookies::Cookies;

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };

    let cookies: Cookies = match extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Ok(false);
    };

    match sqlx::query(
        "SELECT 1 FROM web_sessions WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(row) => {
            let logged_in = row.is_some();
            if logged_in {
                leptos_axum::redirect("/guilds");
            }
            Ok(logged_in)
        },
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[cfg(feature = "ssr")]
#[must_use]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    use leptos::hydration::{AutoReload, HydrationScripts};
    use leptos_meta::MetaTags;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[expect(
    clippy::must_use_candidate,
    reason = "Leptos component; return value is consumed by the view system"
)]
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/dashboard.css"/>
        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/login") view=LoginPage/>
                <ParentRoute path=path!("/") view=Layout>
                    <Route path=path!("") view=Home/>
                    <Route path=path!("guilds") view=GuildListPage/>
                    <Route path=path!("guild/:id/settings") view=GuildSettingsPage/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

/// Top nav-bar + collapsible sidebar wrapping the active child route via
/// `<Outlet/>`.
#[component]
fn Layout() -> impl IntoView {
    view! {
        <div class="layout">
            <Title text="Zayden Dashboard"/>
            <NavBar/>
            <div class="layout-body">
                <Sidebar/>
                <main class="layout-main">
                    <Outlet/>
                </main>
            </div>
        </div>
    }
}

#[component]
fn NavBar() -> impl IntoView {
    view! {
        <nav class="navbar">
            <A href="/" attr:class="navbar-brand">"Zayden"</A>
            <div class="navbar-links">
                <A href="/guilds">"Servers"</A>
                <TierBadge/>
            </div>
        </nav>
    }
}

/// Client-hydrated tier badge.  Fetches the current user's billing tier via a
/// non-blocking server function so it does not delay the initial SSR render of
/// any page.  Renders nothing while loading or when the user is unauthenticated.
/// Shows an "Upgrade to Pro" link alongside the "Free" badge when the dashboard
/// has an `upgrade_url` configured.
#[component]
fn TierBadge() -> impl IntoView {
    let tier_info = Resource::new(|| (), |()| get_user_tier());

    view! {
        <Suspense fallback=|| ()>
            {move || {
                tier_info
                    .get()
                    .and_then(Result::ok)
                    .filter(|i| !i.tier.is_empty())
                    .map(|info| {
                        let label = match info.tier.as_str() {
                            "pro" => "Pro",
                            "enterprise" => "Enterprise",
                            _ => "Free",
                        };
                        let is_free = info.tier == "free";
                        let tier_cls = format!("tier-badge tier-{}", info.tier);
                        view! {
                            <span class=tier_cls>{label}</span>
                            {info.upgrade_url.filter(|_| is_free).map(|url| view! {
                                <a
                                    href=url
                                    class="btn-upgrade"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                >"Upgrade to Pro"</a>
                            })}
                        }
                        .into_any()
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    let open = RwSignal::new(true);

    view! {
        <aside class="sidebar">
            <button
                class="sidebar-toggle"
                on:click=move |_| open.update(|v| *v = !*v)
            >
                {move || if open.get() { "←" } else { "→" }}
            </button>
            <Show when=move || open.get()>
                <nav class="sidebar-nav">
                    <A href="/guilds">"Servers"</A>
                </nav>
            </Show>
        </aside>
    }
}

#[component]
fn Home() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"Dashboard"</h1>
            <p>"Select a server from the sidebar to manage its settings."</p>
        </div>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    // Blocking resource: SSR waits for this before sending HTML.
    // If the session is valid, check_session calls leptos_axum::redirect("/guilds")
    // server-side, so logged-in visitors are redirected before seeing this page.
    let session = Resource::new_blocking(|| (), |()| check_session());

    view! {
        <Title text="Sign In — Zayden Dashboard"/>
        // When the resource resolves as logged-in, render <Redirect> which fires
        // the 302 on SSR (via ServerRedirectFunction) and navigates client-side.
        <Suspense fallback=|| ()>
            {move || {
                session.get()
                    .and_then(Result::ok)
                    .filter(|&logged_in| logged_in)
                    .map(|_| view! { <Redirect path="/guilds"/> })
            }}
        </Suspense>
        <div class="login-page">
            <h1>"Sign in to Zayden Dashboard"</h1>
            <p>"Connect your Discord account to manage server settings."</p>
            <a href="/auth/discord">"Sign in with Discord"</a>
        </div>
    }
}

#[component]
fn GuildListPage() -> impl IntoView {
    // Blocking resource: SSR waits for the guild list before sending HTML so
    // the unauthenticated redirect fires before any page content is sent.
    let guilds = Resource::new_blocking(|| (), |()| list_manageable_guilds());

    view! {
        <Title text="Servers — Zayden Dashboard"/>
        <div class="page">
            <h1>"Your Servers"</h1>
            <Suspense fallback=|| view! { <p class="loading">"Loading servers\u{2026}"</p> }>
                {move || guilds.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="error">"Failed to load servers: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(list) if list.is_empty() => view! {
                        <p class="empty">"You manage no servers with this account."</p>
                    }.into_any(),
                    Ok(list) => view! {
                        <ul class="guild-list">
                            {list.into_iter().map(|g| {
                                let icon_url = g.icon.map(|hash| {
                                    format!(
                                        "https://cdn.discordapp.com/icons/{}/{}.png?size=64",
                                        g.id, hash,
                                    )
                                });
                                let href = format!("/guild/{}/settings", g.id);
                                view! {
                                    <li class="guild-item">
                                        <A href=href>
                                            {icon_url.map(|url| view! {
                                                <img src=url alt="" class="guild-icon"/>
                                            })}
                                            <span class="guild-name">{g.name}</span>
                                        </A>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn GuildSettingsPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    // Load settings once on mount; a simple page reload re-fetches after edits.
    let settings = Resource::new_blocking(guild_id, get_guild_settings);

    // One ServerAction per fieldset keeps each below the 7-argument lint limit.
    let save_support = ServerAction::<SaveSupportSettings>::new();
    let save_channels = ServerAction::<SaveChannelSettings>::new();
    let save_roles = ServerAction::<SaveRoleSettings>::new();
    let save_temp_voice = ServerAction::<SaveTempVoiceSettings>::new();
    let save_lfg = ServerAction::<SaveLfgSettings>::new();

    view! {
        <Title text="Settings — Zayden Dashboard"/>
        <div class="page">
            <h1>"Server Settings"</h1>
            <Suspense fallback=|| view! { <p class="loading">"Loading settings\u{2026}"</p> }>
                {move || settings.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="error">"Failed to load settings: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(s) => {
                        let is_pro = s.is_pro;
                        view! {
                            <Show when=move || !is_pro>
                                <div class="banner-pro">
                                    "This server is on the Free tier. "
                                    "Upgrade to Pro to save settings."
                                </div>
                            </Show>

                            // Support
                            {let r = save_support.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"Support"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_support>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="Support Channel ID"
                                            name="support_channel_id"
                                            value=s.support_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Support Role ID"
                                            name="support_role_id"
                                            value=s.support_role_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="FAQ Channel ID"
                                            name="faq_channel_id"
                                            value=s.faq_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Suggestions Channel ID"
                                            name="suggestions_channel_id"
                                            value=s.suggestions_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Review Channel ID"
                                            name="review_channel_id"
                                            value=s.review_channel_id.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}

                            // Channels
                            {let r = save_channels.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"Channels"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_channels>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="Rules Channel ID"
                                            name="rules_channel_id"
                                            value=s.rules_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="General Channel ID"
                                            name="general_channel_id"
                                            value=s.general_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Spoiler Channel ID"
                                            name="spoiler_channel_id"
                                            value=s.spoiler_channel_id.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}

                            // Roles
                            {let r = save_roles.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"Roles"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_roles>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="Artist Role ID"
                                            name="artist_role_id"
                                            value=s.artist_role_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Sleep Role ID"
                                            name="sleep_role_id"
                                            value=s.sleep_role_id.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}

                            // Temp Voice
                            {let r = save_temp_voice.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"Temp Voice"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_temp_voice>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="Category ID"
                                            name="temp_voice_category"
                                            value=s.temp_voice_category.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Creator Channel ID"
                                            name="temp_voice_creator_channel"
                                            value=s.temp_voice_creator_channel.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}

                            // LFG
                            {let r = save_lfg.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"LFG"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_lfg>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="LFG Channel ID"
                                            name="lfg_channel_id"
                                            value=s.lfg_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="LFG Role ID"
                                            name="lfg_role_id"
                                            value=s.lfg_role_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="LFG Scheduled Thread ID"
                                            name="lfg_scheduled_thread_id"
                                            value=s.lfg_scheduled_thread_id.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}
                        }.into_any()
                    },
                })}
            </Suspense>
        </div>
    }
}

fn save_feedback(r: Result<(), ServerFnError>) -> AnyView {
    match r {
        Ok(()) => view! { <p class="success">"Saved."</p> }.into_any(),
        Err(ref e) if e.to_string().contains("pro_required") => view! {
            <p class="error">"A Pro subscription is required to save settings."</p>
        }
        .into_any(),
        Err(e) => view! { <p class="error">"Failed to save: " {e.to_string()}</p> }
            .into_any(),
    }
}

#[component]
fn SaveButton(is_pro: bool) -> impl IntoView {
    view! {
        <div class="form-actions">
            <button type="submit" class="btn-save" disabled=move || !is_pro>
                {if is_pro { "Save" } else { "Pro Required" }}
            </button>
        </div>
    }
}

/// A labelled text input for a single snowflake ID setting.
#[component]
fn SettingField(
    label: &'static str,
    name: &'static str,
    value: String,
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label>{label}</label>
            <input
                type="text"
                name=name
                value=value
                placeholder="(not set)"
                pattern="[0-9]*"
            />
        </div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"404"</h1>
            <p>"Page not found."</p>
        </div>
    }
}
