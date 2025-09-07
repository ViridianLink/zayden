pub mod error;
mod web;
pub use error::{Error, Result};

use axum::extract::State;
use axum::http::{HeaderValue, Method, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Router, middleware};
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, EndpointSet, RedirectUrl, Scope, TokenUrl,
};
use oauth2::{EndpointNotSet, basic::BasicClient};
use std::net::SocketAddr;
use std::path::Path;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::CorsLayer;

const FRONTEND_URL: &str = "http://localhost:5173";
const CLIENT_ID: &str = "787490197943091211";
const REDIRECT_URI: &str = "http://localhost:3000/auth/callback";

#[derive(Clone)]
struct AppState {
    http_client: reqwest::Client,
    oauth_client:
        BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
}

impl AppState {
    pub fn new(client_secret: String) -> Self {
        let http_client = reqwest::ClientBuilder::new().build().unwrap();

        let oauth_client = BasicClient::new(ClientId::new(CLIENT_ID.to_string()))
            .set_client_secret(ClientSecret::new(client_secret))
            .set_auth_uri(AuthUrl::new("https://discord.com/oauth2/authorize".to_string()).unwrap())
            .set_token_uri(
                TokenUrl::new("https://discord.com/api/oauth2/token".to_string()).unwrap(),
            )
            .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.to_string()).unwrap());

        Self {
            http_client,
            oauth_client,
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
    if let Err(dotenvy::Error::Io(_)) = dotenvy::dotenv() {
        dotenvy::from_path(Path::new("web-backend/.env")).unwrap()
    }

    let client_secret =
        std::env::var("DISCORD_CLIENT_SECRET").expect("Missing DISCORD_CLIENT_SECRET");

    let state = AppState::new(client_secret);

    let cors = CorsLayer::new()
        .allow_origin(FRONTEND_URL.parse::<HeaderValue>().unwrap())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .route("/invite", get(invite_handler))
        .route("/login", get(login_handler))
        .merge(web::routes())
        .layer(middleware::map_response(main_response_mapper))
        .layer(cors)
        .layer(CookieManagerLayer::new())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Dashboard listening on http://{addr}");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
