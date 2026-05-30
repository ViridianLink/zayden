use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, ClientBuilder};
use url::Url;

use crate::chat::ResponseBody;
use crate::error::AiError as Error;
use crate::output::Response;

#[derive(Debug)]
pub struct OpenAI {
    api_url: Url,
    client: Client,
}

impl OpenAI {
    pub fn new(api_key: impl Into<String>) -> Result<Self, Error> {
        let api_key = api_key.into();

        let mut headers = HeaderMap::with_capacity(2);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|e| Error::InvalidHeader(e.to_string()))?,
        );

        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .map_err(Error::Reqwest)?;

        let api_url = Url::parse("https://api.openai.com/v1/")
            .map_err(|e| Error::InvalidUrl(e.to_string()))?;

        Ok(Self { api_url, client })
    }

    pub async fn create_response(
        &self,
        body: &ResponseBody,
    ) -> Result<Response, Error> {
        let url = self
            .api_url
            .join("responses")
            .map_err(|e| Error::InvalidUrl(e.to_string()))?;

        let body_str = serde_json::to_string(body)
            .map_err(|e| Error::ParseResponse { source: e, body: String::new() })?;

        let text = self.client.post(url).body(body_str).send().await?.text().await?;

        serde_json::from_str(&text)
            .map_err(|source| Error::ParseResponse { source, body: text })
    }
}
