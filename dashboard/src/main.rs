pub mod error;
pub mod middleware;
pub mod state;
pub mod web;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use dashboard::app::{App, UpgradeUrl, shell};
use dashmap::DashMap;
pub use error::{Error, Result};
use leptos::config::{LeptosOptions, get_configuration};
use leptos::prelude::provide_context;
use leptos_axum::{LeptosRoutes, generate_route_list};
use moka::future::Cache;
use oauth2::basic::BasicClient;
use oauth2::url::ParseError;
use oauth2::{CsrfToken, EndpointNotSet, EndpointSet, Scope};
use reqwest::header::AUTHORIZATION;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, fmt};
use zayden_app::config::BotConfig;
use zayden_app::events::listener::EventListener;
use zayden_app::state::AppState as ZaydenAppState;

/// Web-backend-specific state wrapping the shared [`ZaydenAppState`] plus
/// OAuth and Discord HTTP fields that are only needed by the dashboard process.
#[derive(Clone)]
pub(crate) struct WebState {
    pub(crate) app: Arc<ZaydenAppState>,
    pub(crate) oauth_client: BasicClient<
        EndpointSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointSet,
    >,
    pub(crate) http_oauth: oauth2::reqwest::Client,
    pub(crate) discord_token: String,
    pub(crate) oauth_states: Arc<DashMap<String, Instant>>,
    pub(crate) frontend_url: String,
    pub(crate) invite_url: Option<String>,
    pub(crate) bot_owner: u64,
    pub(crate) upgrade_url: Option<String>,
    pub(crate) session_cache: Cache<String, (String, i64)>,
    pub(crate) guild_cache:
        Cache<String, Arc<[middleware::guild_permission::PartialGuild]>>,
    pub(crate) leptos_options: LeptosOptions,
}

impl WebState {
    pub(crate) fn new(
        app: Arc<ZaydenAppState>,
        config: &BotConfig,
        leptos_options: LeptosOptions,
    ) -> std::result::Result<Self, ParseError> {
        Ok(Self {
            app,
            oauth_client: state::build_oauth_client(config)?,
            http_oauth: oauth2::reqwest::Client::new(),
            discord_token: config.discord_token.clone(),
            oauth_states: Arc::new(DashMap::new()),
            bot_owner: config.bot_owner,
            upgrade_url: config.upgrade_url.clone(),
            frontend_url: config.frontend_url.clone(),
            invite_url: config.invite_url.clone(),
            session_cache: Cache::builder()
                .max_capacity(1024)
                .time_to_live(Duration::from_mins(1))
                .build(),
            guild_cache: Cache::builder()
                .max_capacity(1024)
                .time_to_live(Duration::from_mins(1))
                .build(),
            leptos_options,
        })
    }
}

impl FromRef<WebState> for LeptosOptions {
    fn from_ref(state: &WebState) -> Self {
        state.leptos_options.clone()
    }
}

// TODO: Remove Box<dyn Error>
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    logging();

    if let Err(dotenvy::Error::Io(_)) = dotenvy::dotenv()
        && dotenvy::from_path(Path::new("web-backend/.env")).is_err()
    {
        warn!(".env file not found. Please make sure enviroment variables are set.");
    }

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    let config = BotConfig::load(&pool).await?;

    let leptos_options = get_configuration(Some("Cargo.toml"))?.leptos_options;

    let app_state = Arc::new(ZaydenAppState::new(pool, &config));
    EventListener::spawn(app_state.db.clone(), app_state.events.clone());
    let web_state = WebState::new(Arc::clone(&app_state), &config, leptos_options)?;

    let cors_origin = HeaderValue::from_str(&config.frontend_url)?;
    let cors = CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_methods([Method::GET, Method::POST, Method::PATCH])
        .allow_headers([AUTHORIZATION]);

    let routes = generate_route_list(App);

    let app: Router = Router::new()
        .route("/invite", get(invite_handler))
        // /auth/discord starts the OAuth2 flow; /login is now the Leptos page.
        .route("/auth/discord", get(login_handler))
        .merge(web::routes(web_state.clone()))
        .leptos_routes_with_context(&web_state, routes, {
            let db = web_state.app.db.clone();
            let http = web_state.app.http.clone();
            let app = Arc::clone(&web_state.app);
            let upgrade_url = web_state.upgrade_url.clone();
            move || {
                provide_context(db.clone());
                provide_context(http.clone());
                provide_context(Arc::clone(&app));
                provide_context(UpgradeUrl(upgrade_url.clone()));
            }
        }, {
            let lo = web_state.leptos_options.clone();
            move || shell(lo.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<WebState, _>(shell))
        .layer(cors)
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

async fn invite_handler(State(state): State<WebState>) -> Response {
    state.invite_url.as_deref().map_or_else(
        || StatusCode::NOT_FOUND.into_response(),
        |url| Redirect::to(url).into_response(),
    )
}

async fn login_handler(State(state): State<WebState>) -> impl IntoResponse {
    let (auth_url, csrf_token) = state
        .oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scopes([
            Scope::new("identify".to_string()),
            Scope::new("guilds".to_string()),
            Scope::new("email".to_string()),
            Scope::new("applications.commands.permissions.update".to_string()),
        ])
        .url();

    state.oauth_states.insert(csrf_token.secret().clone(), Instant::now());

    Redirect::to(auth_url.as_str())
}
