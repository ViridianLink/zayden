use axum::Router;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointSet, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use oauth2::{EndpointNotSet, basic::BasicClient};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

const CLIENT_ID: &str = "787490197943091211";
const REDIRECT_URI: &str = "http://127.0.0.1:3000/auth/callback";

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

#[tokio::main]
async fn main() {
    if dotenvy::dotenv().is_err() {
        println!(".env file not found. Please make sure enviroment variables are set.")
    }

    let client_secret =
        std::env::var("DISCORD_CLIENT_SECRET").expect("Missing DISCORD_CLIENT_SECRET");

    let state = AppState::new(client_secret);

    let app = Router::new()
        .route("/invite", get(invite_handler))
        .route("/login", get(login_handler))
        .route("/auth/callback", get(auth_callback_handler))
        .fallback_service(ServeDir::new("web/public"))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Dashboard listening on http://{addr}");

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
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

#[derive(Deserialize)]
struct AuthCallbackQuery {
    code: String,
    #[allow(dead_code)]
    state: String, // You should use and verify the CSRF token in a real app
}

// Handler for the Discord redirect
async fn auth_callback_handler(
    Query(query): Query<AuthCallbackQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let token_result = state
        .oauth_client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(&state.http_client)
        .await;

    if let Ok(token) = token_result {
        println!("Got token: {:?}", token.access_token().secret());

        // You would now use this token to fetch user data from the Discord API.
        // Then, you'd create a session (e.g., using a JWT or a session cookie)
        // and store the user's info.
        // For this example, we'll just redirect to the dashboard.

        // This is where you would set a session cookie.
        // Using a library like `axum-sessions` or `tower-cookies` is recommended.
        Redirect::to("/dashboard.html")
    } else {
        Redirect::to("/?error=auth_failed")
    }
}
