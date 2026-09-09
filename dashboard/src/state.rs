use std::sync::Arc;
use std::time::Duration;

use axum::extract::FromRef;
use dashboard::server::auth::{SessionCache, UserGuildsCache};
use leptos::config::LeptosOptions;
use moka::future::Cache;
use oauth2::basic::BasicClient;
use oauth2::url::ParseError;
use oauth2::{
    AuthUrl,
    ClientId,
    ClientSecret,
    EndpointNotSet,
    EndpointSet,
    RedirectUrl,
    TokenUrl,
};
use patreon::oauth::PatreonApp;
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

const DISCORD_OAUTH_AUTH_URL: &str = "https://discord.com/oauth2/authorize";
const DISCORD_OAUTH_TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const CACHE_CAPACITY: u64 = 1024;
const CACHE_TTL: Duration = Duration::from_mins(1);

type DiscordOAuthClient = BasicClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

pub(crate) struct OAuthState {
    pub(crate) client: DiscordOAuthClient,
    pub(crate) http: oauth2::reqwest::Client,
}

#[derive(Clone)]
pub(crate) struct DiscordState {
    pub(crate) http: Arc<twilight_http::Client>,
    pub(crate) user_guilds: UserGuildsCache,
}

pub(crate) struct IntegrationsState {
    pub(crate) patreon: Option<PatreonApp>,
    pub(crate) patreon_webhook_uri: String,
    pub(crate) kofi_verification_token: Option<String>,
}

pub(crate) struct SiteUrls {
    pub(crate) invite: Option<String>,
    pub(crate) upgrade: Option<String>,
}

#[derive(Clone)]
pub(crate) struct SessionState {
    pub(crate) app: Arc<ZaydenAppState>,
    pub(crate) cache: SessionCache,
}

#[derive(Clone)]
pub(crate) struct WebState {
    pub(crate) app: Arc<ZaydenAppState>,
    pub(crate) sessions: SessionCache,
    pub(crate) oauth: Arc<OAuthState>,
    pub(crate) discord: DiscordState,
    pub(crate) integrations: Arc<IntegrationsState>,
    pub(crate) urls: Arc<SiteUrls>,
    pub(crate) leptos_options: LeptosOptions,
}

impl WebState {
    pub(crate) fn new(
        app: Arc<ZaydenAppState>,
        config: &BotConfig,
        leptos_options: LeptosOptions,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            app,
            sessions: cache(),
            oauth: Arc::new(OAuthState {
                client: build_oauth_client(config)?,
                http: oauth2::reqwest::Client::new(),
            }),
            discord: DiscordState {
                http: Arc::new(twilight_http::Client::new(
                    config.discord_token.clone(),
                )),
                user_guilds: cache(),
            },
            integrations: Arc::new(IntegrationsState {
                patreon: config.patreon.as_ref().map(|p| PatreonApp {
                    client_id: p.client_id.clone(),
                    client_secret: p.client_secret.clone(),
                    redirect_uri: p.redirect_uri.clone(),
                }),
                // Patreon delivers to a fixed URL, so it is derived from the
                // callback the app is already registered against.
                patreon_webhook_uri: config.patreon.as_ref().map_or_else(
                    String::new,
                    |p| {
                        p.redirect_uri
                            .replace("/patreon/callback", "/webhooks/patreon")
                    },
                ),
                kofi_verification_token: config.kofi_verification_token.clone(),
            }),
            urls: Arc::new(SiteUrls {
                invite: config.invite_url.clone(),
                upgrade: config.upgrade_url.clone(),
            }),
            leptos_options,
        })
    }
}

impl FromRef<WebState> for Arc<ZaydenAppState> {
    fn from_ref(state: &WebState) -> Self {
        Self::clone(&state.app)
    }
}

impl FromRef<WebState> for SessionState {
    fn from_ref(state: &WebState) -> Self {
        Self { app: Arc::clone(&state.app), cache: state.sessions.clone() }
    }
}

impl FromRef<WebState> for Arc<OAuthState> {
    fn from_ref(state: &WebState) -> Self {
        Self::clone(&state.oauth)
    }
}

impl FromRef<WebState> for DiscordState {
    fn from_ref(state: &WebState) -> Self {
        state.discord.clone()
    }
}

impl FromRef<WebState> for Arc<IntegrationsState> {
    fn from_ref(state: &WebState) -> Self {
        Self::clone(&state.integrations)
    }
}

impl FromRef<WebState> for Arc<SiteUrls> {
    fn from_ref(state: &WebState) -> Self {
        Self::clone(&state.urls)
    }
}

impl FromRef<WebState> for LeptosOptions {
    fn from_ref(state: &WebState) -> Self {
        state.leptos_options.clone()
    }
}

fn cache<K, V>() -> Cache<K, V>
where
    K: Send + Sync + Eq + std::hash::Hash + 'static,
    V: Send + Sync + Clone + 'static,
{
    Cache::builder().max_capacity(CACHE_CAPACITY).time_to_live(CACHE_TTL).build()
}

fn build_oauth_client(config: &BotConfig) -> Result<DiscordOAuthClient, ParseError> {
    Ok(BasicClient::new(ClientId::new(config.zayden_id.to_string()))
        .set_client_secret(ClientSecret::new(config.discord_client_secret.clone()))
        .set_auth_uri(AuthUrl::new(DISCORD_OAUTH_AUTH_URL.to_string())?)
        .set_token_uri(TokenUrl::new(DISCORD_OAUTH_TOKEN_URL.to_string())?)
        .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?))
}
