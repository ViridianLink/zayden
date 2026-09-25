use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tracing::warn;
use url::Url;

use crate::error::{Result, YoutubeError};

const AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";

pub const SCOPES: &str = "https://www.googleapis.com/auth/youtube.readonly";

#[derive(Debug, Clone)]
pub struct YoutubeApp {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

impl YoutubeApp {
    pub fn authorize_url(&self, state: &str) -> Result<String> {
        let mut url = Url::parse(AUTHORIZE_URL)
            .map_err(|e| YoutubeError::Internal(e.to_string()))?;

        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", SCOPES)
            .append_pair("access_type", "online")
            .append_pair("prompt", "select_account")
            .append_pair("state", state);

        Ok(url.into())
    }

    pub async fn exchange_code(
        &self,
        client: &Client,
        code: &str,
    ) -> Result<String> {
        let response = client
            .post(TOKEN_ENDPOINT)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("redirect_uri", &self.redirect_uri),
            ])
            .send()
            .await?;

        // Google answers a spent or forged code with 400 `invalid_grant`.
        if matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::BAD_REQUEST
        ) {
            return Err(YoutubeError::Unauthorized);
        }

        let token = response.error_for_status()?.json::<TokenResponse>().await?;

        Ok(token.access_token)
    }
}

pub async fn revoke(client: &Client, access_token: &str) {
    let result = client
        .post(REVOKE_ENDPOINT)
        .form(&[("token", access_token)])
        .send()
        .await
        .and_then(reqwest::Response::error_for_status);

    if let Err(e) = result {
        warn!(error = ?e, "youtube: failed to revoke the connect grant");
    }
}
