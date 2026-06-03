use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl,
    ClientId,
    ClientSecret,
    EndpointNotSet,
    EndpointSet,
    RedirectUrl,
    TokenUrl,
};
use zayden_app::config::BotConfig;

const DISCORD_OAUTH_AUTH_URL: &str = "https://discord.com/oauth2/authorize";
const DISCORD_OAUTH_TOKEN_URL: &str = "https://discord.com/api/oauth2/token";

/// Construct an `OAuth2` client pointed at Discord's stable authorization and
/// token endpoints using deployment-specific settings from `config`.
pub(crate) fn build_oauth_client(
    config: &BotConfig,
) -> BasicClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
> {
    BasicClient::new(ClientId::new(config.zayden_id.to_string()))
        .set_client_secret(ClientSecret::new(config.discord_client_secret.clone()))
        .set_auth_uri(
            AuthUrl::new(DISCORD_OAUTH_AUTH_URL.to_string())
                .expect("static OAuth2 auth URL is valid"),
        )
        .set_token_uri(
            TokenUrl::new(DISCORD_OAUTH_TOKEN_URL.to_string())
                .expect("static OAuth2 token URL is valid"),
        )
        .set_redirect_uri(
            RedirectUrl::new(config.redirect_uri.clone())
                .expect("BotConfig::redirect_uri is a valid URL"),
        )
}
