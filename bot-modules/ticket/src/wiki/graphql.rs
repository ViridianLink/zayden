use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::wiki::{WikiConfig, WikiError};

#[derive(Deserialize)]
struct Envelope<T> {
    #[serde(default = "Option::default")]
    data: Option<T>,
    #[serde(default = "Vec::new")]
    errors: Vec<GraphQlError>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
    #[serde(default = "Option::default")]
    extensions: Option<Extensions>,
}

#[derive(Deserialize)]
struct Extensions {
    #[serde(default = "Option::default")]
    exception: Option<Exception>,
}

#[derive(Deserialize)]
struct Exception {
    #[serde(default = "Option::default")]
    code: Option<i64>,
}

const PAGE_VIEW_FORBIDDEN: i64 = 6013;

pub(crate) async fn query<T: DeserializeOwned>(
    client: &Client,
    config: &WikiConfig,
    body: &serde_json::Value,
) -> Result<T, WikiError> {
    let mut request = client.post(config.graphql_endpoint()).json(body);

    if let Some(key) = config.api_key() {
        request = request.bearer_auth(key);
    }

    let response = request.send().await?;
    let status = response.status();
    let envelope: Envelope<T> = response.json().await?;

    if let Some(error) = envelope.errors.first() {
        let code = error
            .extensions
            .as_ref()
            .and_then(|e| e.exception.as_ref())
            .and_then(|e| e.code);

        if code == Some(PAGE_VIEW_FORBIDDEN) {
            return Err(WikiError::PageForbidden);
        }

        return Err(WikiError::GraphQl {
            status: status.as_u16(),
            message: error.message.clone(),
        });
    }

    envelope.data.ok_or(WikiError::EmptyResponse)
}
