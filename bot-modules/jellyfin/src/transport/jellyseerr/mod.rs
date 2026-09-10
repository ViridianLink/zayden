pub mod discover;
pub mod model;
pub mod request;
pub mod search;

use reqwest::{Client, RequestBuilder};

use crate::transport::http::trim_base_url;

pub const SERVICE: &str = "Jellyseerr";

#[derive(Debug, Clone)]
pub struct SeerClient {
    client: Client,
    base_url: String,
    api_key: String,
    region: String,
}

impl SeerClient {
    #[must_use]
    pub fn new(
        client: Client,
        base_url: String,
        api_key: String,
        region: String,
    ) -> Self {
        Self { client, base_url: trim_base_url(base_url), api_key, region }
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    #[must_use]
    pub fn region(&self) -> &str {
        &self.region
    }

    pub(crate) fn get(&self, path: &str) -> RequestBuilder {
        self.authed(self.client.get(self.endpoint(path)))
    }

    pub(crate) fn post(&self, path: &str) -> RequestBuilder {
        self.authed(self.client.post(self.endpoint(path)))
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}/api/v1/{path}", self.base_url)
    }

    fn authed(&self, builder: RequestBuilder) -> RequestBuilder {
        builder
            .header("X-Api-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
    }
}
