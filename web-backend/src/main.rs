pub mod error;
pub mod middleware;
pub mod web;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::middleware::map_response;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
pub use error::{Error, Result};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl,
    ClientId,
    ClientSecret,
    CsrfToken,
    EndpointNotSet,
    EndpointSet,
    RedirectUrl,
    Scope,
    TokenUrl,
};
use reqwest::header::AUTHORIZATION;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::{Any, CorsLayer};
use tracing::warn;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, filter, fmt};
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

const FRONTEND_URL: &str = "http://localhost:5173";

const REDIRECT_URI: &str = if cfg!(debug_assertions) {
    "http://localhost:3000/auth/callback"
} else {
    "http://145.40.184.89:80/auth/callback"
};

/// Web-backend-specific state wrapping the shared [`ZaydenAppState`] plus
/// OAuth and Discord HTTP fields that are only needed by the dashboard process.
#[derive(Clone)]
pub(crate) struct WebState {
    pub(crate) app: Arc<ZaydenAppState>,
    oauth_client: BasicClient<
        EndpointSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointSet,
    >,
    pub(crate) discord_token: String,
}

impl WebState {
    pub(crate) fn new(app: Arc<ZaydenAppState>, config: &BotConfig) -> Self {
        let oauth_client =
            BasicClient::new(ClientId::new(config.zayden_id.to_string()))
                .set_client_secret(ClientSecret::new(
                    config.discord_client_secret.clone(),
                ))
                .set_auth_uri(
                    AuthUrl::new("https://discord.com/oauth2/authorize".to_string())
                        .expect("static OAuth2 auth URL is valid"),
                )
                .set_token_uri(
                    TokenUrl::new(
                        "https://discord.com/api/oauth2/token".to_string(),
                    )
                    .expect("static OAuth2 token URL is valid"),
                )
                .set_redirect_uri(
                    RedirectUrl::new(REDIRECT_URI.to_string())
                        .expect("static redirect URI is valid"),
                );

        Self { app, oauth_client, discord_token: config.discord_token.clone() }
    }
}

async fn main_response_mapper(res: Response) -> Response {
    println!("->> {:<12} - main_response_mapper", "RES_MAPPER");
    println!();
    res
}

#[tokio::main]
async fn main() {
    logging();

    if let Err(dotenvy::Error::Io(_)) = dotenvy::dotenv()
        && dotenvy::from_path(Path::new("web-backend/.env")).is_err()
    {
        warn!(".env file not found. Please make sure enviroment variables are set.");
    }

    let database_url = std::env::var("DATABASE_URL").expect("Missing DATABASE_URL");
    let pool =
        PgPool::connect(&database_url).await.expect("failed to connect to database");

    let config = BotConfig::load(&pool).await.expect("failed to load BotConfig");

    let app_state = Arc::new(ZaydenAppState::new(pool, &config));
    let state = WebState::new(Arc::clone(&app_state), &config);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers([AUTHORIZATION]);

    let app: Router = Router::new()
        .route("/invite", get(invite_handler))
        .route("/login", get(login_handler))
        .merge(web::routes(state.clone()))
        .layer(map_response(main_response_mapper))
        .layer(cors)
        .layer(CookieManagerLayer::new())
        .with_state(state);

    let ip = if cfg!(debug_assertions) { [127, 0, 0, 1] } else { [0, 0, 0, 0] };

    let addr = SocketAddr::from((ip, 3000));
    println!("Dashboard listening on http://{addr}");

    let listener = TcpListener::bind(addr).await.expect();
    axum::serve(listener, app).await.expect();
}

fn logging() {
    let stdout_log =
        fmt::layer().with_writer(io::stdout).with_filter(LevelFilter::INFO);

    Registry::default().with(stdout_log).init();
}

async fn invite_handler() -> impl IntoResponse {
    const INVITE_URL: &str = "https://discord.com/oauth2/authorize?client_id=787490197943091211&permissions=8&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A3000%2Fauth%2Fcallback&integration_type=0&scope=identify+bot+guilds+applications.commands";

    Redirect::to(INVITE_URL)
}

async fn login_handler(State(state): State<WebState>) -> impl IntoResponse {
    let (auth_url, _csrf_token) = state
        .oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scopes([
            Scope::new("identify".to_string()),
            Scope::new("guilds".to_string()),
            Scope::new("email".to_string()),
            Scope::new("applications.commands.permissions.update".to_string()),
        ])
        .url();

    Redirect::to(auth_url.as_str())
}
