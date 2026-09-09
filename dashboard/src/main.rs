#![recursion_limit = "256"]

pub mod middleware;
pub mod state;
pub mod web;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use dashboard::app::{App, UpgradeUrl, shell};
use leptos::config::get_configuration;
use leptos::prelude::provide_context;
use leptos_axum::{LeptosRoutes, generate_route_list};
use oauth2::{CsrfToken, Scope};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_cookies::{CookieManagerLayer, Cookies};
use tracing::{info, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, fmt};
use zayden_app::config::BotConfig;
use zayden_app::events::listener::EventListener;
use zayden_app::state::AppState as ZaydenAppState;

use crate::state::{OAuthState, SiteUrls, WebState};
use crate::web::cookie::{self, OAUTH_STATE_COOKIE};

const SESSION_PRUNE_INTERVAL: Duration = Duration::from_hours(1);
const OAUTH_STATE_TTL: Duration = Duration::from_mins(10);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging();

    if rustls::crypto::aws_lc_rs::default_provider().install_default().is_err() {
        warn!("Rustls CryptoProvider was already installed");
    }

    let _ = dotenvy::dotenv();

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    let config = BotConfig::load(&pool).await?;

    let leptos_options =
        get_configuration(Some("dashboard/Cargo.toml"))?.leptos_options;

    let app_state = Arc::new(ZaydenAppState::new(pool, &config));
    EventListener::spawn(app_state.db.clone(), app_state.events.clone());
    let web_state = WebState::new(Arc::clone(&app_state), &config, leptos_options)?;

    tokio::spawn({
        let pool = web_state.app.db.clone();
        async move {
            let mut ticker = tokio::time::interval(SESSION_PRUNE_INTERVAL);
            loop {
                ticker.tick().await;
                web::prune_expired_sessions(&pool).await;
            }
        }
    });

    let discord_http = Arc::clone(&web_state.discord.http);

    let routes = generate_route_list(App);

    let app: Router = Router::new()
        .route("/invite", get(invite_handler))
        .route("/auth/discord", get(login_handler))
        .merge(web::routes(&web_state))
        .leptos_routes_with_context(
            &web_state,
            routes,
            {
                let db = web_state.app.db.clone();
                let app = Arc::clone(&web_state.app);
                let upgrade_url = web_state.urls.upgrade.clone();
                let discord_http = Arc::clone(&discord_http);
                let session_cache = web_state.sessions.clone();
                let user_guilds_cache = web_state.discord.user_guilds.clone();
                let patreon = web_state.integrations.patreon.clone();
                move || {
                    provide_context(db.clone());
                    provide_context(Arc::clone(&app));
                    provide_context(UpgradeUrl(upgrade_url.clone()));
                    provide_context(Arc::clone(&discord_http));
                    provide_context(session_cache.clone());
                    provide_context(user_guilds_cache.clone());

                    // Absent when the instance has no Patreon credentials, which
                    // is how `disconnect_patreon` tells "skip the webhook call"
                    // from "the call failed".
                    if let Some(patreon) = patreon.clone() {
                        provide_context(patreon);
                    }
                }
            },
            {
                let lo = web_state.leptos_options.clone();
                move || shell(lo.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler::<WebState, _>(shell))
        .layer(CookieManagerLayer::new())
        .with_state(web_state);

    let addr: SocketAddr = config.bind_addr.parse()?;
    info!("Dashboard listening on {addr}");

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn logging() {
    let stdout_log =
        fmt::layer().with_writer(io::stdout).with_filter(LevelFilter::INFO);

    Registry::default().with(stdout_log).init();
}

async fn invite_handler(State(urls): State<Arc<SiteUrls>>) -> Response {
    urls.invite.as_deref().map_or_else(
        || StatusCode::NOT_FOUND.into_response(),
        |url| Redirect::to(url).into_response(),
    )
}

async fn login_handler(
    cookies: Cookies,
    State(oauth): State<Arc<OAuthState>>,
) -> impl IntoResponse {
    let (auth_url, csrf_token) = oauth
        .client
        .authorize_url(CsrfToken::new_random)
        .add_scopes([
            Scope::new("identify".to_string()),
            Scope::new("guilds".to_string()),
            Scope::new("email".to_string()),
            Scope::new("applications.commands.permissions.update".to_string()),
        ])
        .url();

    let max_age = tower_cookies::cookie::time::Duration::seconds(
        OAUTH_STATE_TTL.as_secs().cast_signed(),
    );
    let state_cookie =
        cookie::build(OAUTH_STATE_COOKIE, csrf_token.secret().clone(), max_age)
            .build();
    cookies.add(state_cookie);

    Redirect::to(auth_url.as_str())
}
