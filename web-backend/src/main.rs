pub mod error;
mod web;
pub use error::{Error, Result};

use axum::extract::State;
use axum::http::{HeaderValue, Method, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Router, middleware};
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, EndpointSet, RedirectUrl, Scope, TokenUrl,
};
use oauth2::{EndpointNotSet, basic::BasicClient};
use reqwest::header::AUTHORIZATION;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::{Any, CorsLayer};
use tracing::warn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, filter, fmt};
use zayden_app::entitlement::EntitlementService;

const FRONTEND_URL: &str = "http://localhost:5173";
const CLIENT_ID: u64 = 787490197943091211;

const REDIRECT_URI: &str = if cfg!(debug_assertions) {
    "http://localhost:3000/auth/callback"
} else {
    "http://145.40.184.89:80/auth/callback"
};

#[derive(Clone)]
struct AppState {
    http_client: reqwest::Client,
    oauth_client:
        BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    discord_client: Arc<twilight_http::Client>,
    entitlements: Arc<EntitlementService>,
}

impl AppState {
    pub fn new(client_secret: String, discord_token: String, pool: PgPool) -> Self {
        let http_client = reqwest::ClientBuilder::new().build().unwrap();

        let oauth_client = BasicClient::new(ClientId::new(CLIENT_ID.to_string()))
            .set_client_secret(ClientSecret::new(client_secret))
            .set_auth_uri(AuthUrl::new("https://discord.com/oauth2/authorize".to_string()).unwrap())
            .set_token_uri(
                TokenUrl::new("https://discord.com/api/oauth2/token".to_string()).unwrap(),
            )
            .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.to_string()).unwrap());

        let discord_client = twilight_http::Client::builder()
            .token(discord_token)
            .build();

        let (events, _) = broadcast::channel(64);
        let entitlements = Arc::new(EntitlementService::new(pool, events.clone()));
        EntitlementService::spawn_invalidator(Arc::clone(&entitlements), events.subscribe());

        Self {
            http_client,
            oauth_client,
            discord_client: Arc::new(discord_client),
            entitlements,
        }
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
        warn!(".env file not found. Please make sure enviroment variables are set.")
    }

    let client_secret =
        std::env::var("DISCORD_CLIENT_SECRET").expect("Missing DISCORD_CLIENT_SECRET");

    let discord_token = std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN");

    let database_url = std::env::var("DATABASE_URL").expect("Missing DATABASE_URL");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let state = AppState::new(client_secret, discord_token, pool);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers([AUTHORIZATION]);

    let app = Router::new()
        .route("/invite", get(invite_handler))
        .route("/login", get(login_handler))
        .route("/webhooks/kofi", post(web::kofi_webhook_handler))
        .merge(web::routes())
        .layer(middleware::map_response(main_response_mapper))
        .layer(cors)
        .layer(CookieManagerLayer::new())
        .with_state(state);

    let ip = if cfg!(debug_assertions) {
        [127, 0, 0, 1]
    } else {
        [0, 0, 0, 0]
    };

    let addr = SocketAddr::from((ip, 3000));
    println!("Dashboard listening on http://{addr}");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn logging() {
    let stdout_log = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(filter::LevelFilter::INFO);

    Registry::default().with(stdout_log).init();
}

async fn invite_handler() -> impl IntoResponse {
    const INVITE_URL: &str = "https://discord.com/oauth2/authorize?client_id=787490197943091211&permissions=8&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A3000%2Fauth%2Fcallback&integration_type=0&scope=identify+bot+guilds+applications.commands";

    Redirect::to(INVITE_URL)
}

async fn login_handler(State(state): State<AppState>) -> impl IntoResponse {
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
