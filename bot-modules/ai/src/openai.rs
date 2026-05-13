use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, ClientBuilder};
use url::Url;

use crate::chat::ResponseBody;
use crate::error::Error;
use crate::output::Response;

#[derive(Debug)]
pub struct OpenAI {
    api_url: Url,
    client: Client,
}

impl OpenAI {
    pub fn new(api_key: impl Into<String>) -> OpenAI {
        let api_key = api_key.into();

        let mut headers = HeaderMap::with_capacity(2);
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str("application/json").unwrap(),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}")).unwrap(),
        );

        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();

        OpenAI {
            api_url: Url::parse("https://api.openai.com/v1/").unwrap(),
            client,
        }
    }

    pub async fn create_response(&self, body: &ResponseBody) -> Result<Response, Error> {
        let text = self
            .client
            .post(self.api_url.join("responses").unwrap())
            .body(serde_json::to_string(body).unwrap())
            .send()
            .await?
            .text()
            .await?;

        serde_json::from_str(&text).map_err(|source| Error::ParseResponse { source, body: text })
    }
}
