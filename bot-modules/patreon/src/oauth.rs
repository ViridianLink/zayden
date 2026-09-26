use jiff::{SignedDuration, Timestamp};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, info, warn};
use url::Url;

use crate::api;
use crate::error::{PatreonError, Result};
use crate::store::PatreonConnection;

const AUTHORIZE_URL: &str = "https://www.patreon.com/oauth2/authorize";
const TOKEN_ENDPOINT: &str = "https://www.patreon.com/api/oauth2/token";

pub const SCOPES: &str = "identity campaigns campaigns.posts w:campaigns.webhook";

const REFRESH_MARGIN: SignedDuration = SignedDuration::from_secs(60);

#[derive(Debug, Clone)]
pub struct PatreonApp {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: f64,
}

impl PatreonApp {
    pub fn authorize_url(&self, state: &str) -> Result<String> {
        let mut url = Url::parse(AUTHORIZE_URL)
            .map_err(|e| PatreonError::Internal(e.to_string()))?;

        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", SCOPES)
            .append_pair("state", state);

        Ok(url.into())
    }

    pub async fn exchange_code(
        &self,
        client: &Client,
        code: &str,
    ) -> Result<TokenPair> {
        self.token_request(client, &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("redirect_uri", &self.redirect_uri),
        ])
        .await
    }

    async fn refresh(
        &self,
        client: &Client,
        refresh_token: &str,
    ) -> Result<TokenPair> {
        self.token_request(client, &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ])
        .await
    }

    async fn token_request(
        &self,
        client: &Client,
        form: &[(&str, &str)],
    ) -> Result<TokenPair> {
        let grant_type = form
            .iter()
            .find_map(|(key, value)| (*key == "grant_type").then_some(*value))
            .unwrap_or_default();

        let response = client.post(TOKEN_ENDPOINT).form(form).send().await?;
        let status = response.status();

        if !status.is_success() {
            let error = response.error_for_status_ref().err();
            let body = response.text().await.unwrap_or_default();
            warn!(
                %status,
                grant_type,
                body = api::truncate_log(&body),
                "patreon: token request failed"
            );

            if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::BAD_REQUEST) {
                return Err(PatreonError::Unauthorized);
            }

            return Err(error.map_or_else(
                || {
                    PatreonError::Payload(format!(
                        "token endpoint answered {status}"
                    ))
                },
                PatreonError::from,
            ));
        }

        let pair = response.json::<TokenPair>().await.inspect_err(|e| {
            error!(error = ?e, grant_type, "patreon: token response did not parse");
        })?;

        Ok(pair)
    }
}

pub async fn access_token(
    pool: &PgPool,
    client: &Client,
    app: &PatreonApp,
    connection: &PatreonConnection,
) -> Result<String> {
    if connection.disabled_at.is_some() {
        return Err(PatreonError::Unauthorized);
    }

    let deadline = connection
        .expires_at
        .to_jiff()
        .checked_sub(REFRESH_MARGIN)
        .unwrap_or(Timestamp::MIN);

    if Timestamp::now() < deadline {
        return Ok(connection.access_token.clone());
    }

    match app.refresh(client, &connection.refresh_token).await {
        Ok(pair) => {
            PatreonConnection::store_tokens(pool, connection.guild_id, &pair)
                .await?;
            info!(guild_id = connection.guild_id, "patreon: access token refreshed");
            Ok(pair.access_token)
        },
        Err(PatreonError::Unauthorized) => {
            PatreonConnection::disable(pool, connection.guild_id).await?;
            error!(
                guild_id = connection.guild_id,
                "patreon: refresh token rejected; the creator must reconnect \
                 from the dashboard"
            );
            Err(PatreonError::Unauthorized)
        },
        Err(e) => Err(e),
    }
}
