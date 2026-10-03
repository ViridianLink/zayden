use std::sync::Arc;
use std::time::Duration;

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
use twilight_model::user::CurrentUserGuild;
use youtube::{YoutubeApp, YoutubeRuntime};
use zayden_app::config::BotConfig;
use zayden_app::state::AppState as ZaydenAppState;

const DISCORD_OAUTH_AUTH_URL: &str = "https://discord.com/oauth2/authorize";
const DISCORD_OAUTH_TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const CACHE_CAPACITY: u64 = 1024;
const CACHE_TTL: Duration = Duration::from_mins(1);

pub type DiscordOAuthClient = BasicClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

#[derive(Clone)]
pub struct SessionIdentity {
    pub user_id: i64,
    pub access_token: String,
}

pub type SessionCache = Cache<String, SessionIdentity>;
pub type UserGuildsCache = Cache<i64, Arc<[CurrentUserGuild]>>;

pub struct OAuthState {
    pub client: DiscordOAuthClient,
    pub http: oauth2::reqwest::Client,
}

pub struct DiscordState {
    pub http: Arc<twilight_http::Client>,
    pub user_guilds: UserGuildsCache,
}

pub struct IntegrationsState {
    pub patreon: Option<PatreonApp>,
    pub patreon_webhook_uri: String,
    pub youtube: Option<YoutubeApp>,
    pub youtube_runtime: Option<YoutubeRuntime>,
    pub kofi_verification_token: Option<String>,
}

pub struct SiteUrls {
    pub invite: Option<String>,
    pub upgrade: Option<String>,
}

pub struct WebState {
    pub app: Arc<ZaydenAppState>,
    pub sessions: SessionCache,
    pub oauth: OAuthState,
    pub discord: DiscordState,
    pub integrations: IntegrationsState,
    pub urls: SiteUrls,
}

impl WebState {
    pub fn new(
        app: Arc<ZaydenAppState>,
        config: &BotConfig,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            app,
            sessions: cache(),
            oauth: OAuthState {
                client: build_oauth_client(config)?,
                http: oauth2::reqwest::Client::new(),
            },
            discord: DiscordState {
                http: Arc::new(twilight_http::Client::new(
                    config.discord_token.clone(),
                )),
                user_guilds: cache(),
            },
            integrations: IntegrationsState {
                patreon: config.patreon.as_ref().map(|p| PatreonApp {
                    client_id: p.client_id.clone(),
                    client_secret: p.client_secret.clone(),
                    redirect_uri: p.redirect_uri.clone(),
                }),
                // Patreon delivers to a fixed URL, so it is derived from the
                // callback the app is already registered against.
                patreon_webhook_uri: config
                    .patreon
                    .as_ref()
                    .map_or_else(String::new, |p| {
                        patreon_webhook_uri(&p.redirect_uri)
                    }),
                youtube: config.youtube.as_ref().map(|y| YoutubeApp {
                    client_id: y.client_id.clone(),
                    client_secret: y.client_secret.clone(),
                    redirect_uri: y.redirect_uri.clone(),
                }),
                youtube_runtime: config.youtube.as_ref().map(|y| YoutubeRuntime {
                    api_key: Arc::from(config.google_api_key.as_str()),
                    webhook_uri: Arc::from(y.webhook_uri.as_str()),
                }),
                kofi_verification_token: config.kofi_verification_token.clone(),
            },
            urls: SiteUrls {
                invite: config.invite_url.clone(),
                upgrade: config.upgrade_url.clone(),
            },
        })
    }
}

#[must_use]
pub fn patreon_webhook_uri(redirect_uri: &str) -> String {
    if !redirect_uri.contains("/patreon/callback") {
        tracing::warn!(
            redirect_uri,
            "Patreon redirect URI has no /patreon/callback path; the webhook URI \
             derived from it will not reach /webhooks/patreon"
        );
    }

    redirect_uri.replace("/patreon/callback", "/webhooks/patreon")
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
