use cynic::{GraphQlResponse, Operation, QueryFragment, QueryVariables};
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::wiki::{WikiConfig, WikiError};

#[derive(Deserialize)]
struct Extensions {
    exception: Option<Exception>,
}

#[derive(Deserialize)]
struct Exception {
    code: Option<i64>,
}

const PAGE_VIEW_FORBIDDEN: i64 = 6013;

pub(crate) async fn run<Query, Variables>(
    client: &Client,
    config: &WikiConfig,
    operation: &Operation<Query, Variables>,
) -> Result<Query, WikiError>
where
    Query: QueryFragment + DeserializeOwned,
    Variables: QueryVariables + serde::Serialize + Sync,
{
    let mut request = client.post(config.graphql_endpoint()).json(operation);

    if let Some(key) = config.api_key() {
        request = request.bearer_auth(key);
    }

    let response = request.send().await?;
    let status = response.status();
    let body: GraphQlResponse<Query, Extensions> = response.json().await?;

    if let Some(error) = body.errors.as_ref().and_then(|errors| errors.first()) {
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

    body.data.ok_or(WikiError::EmptyResponse)
}
