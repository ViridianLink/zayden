pub mod discover;
pub mod model;
pub mod request;
pub mod search;
pub mod service;

use reqwest::{Client, RequestBuilder};
use url::Url;

use crate::error::{JellyfinError, Result};
use crate::transport::http::encode_query;

pub const SERVICE: &str = "Jellyseerr";

#[derive(Debug, Clone)]
pub struct SeerClient {
    client: Client,
    base_url: Url,
    api_key: String,
    region: String,
}

impl SeerClient {
    pub fn new(
        client: Client,
        base_url: &str,
        api_key: String,
        region: String,
    ) -> Result<Self> {
        let url =
            Url::parse(base_url).map_err(|e| JellyfinError::InvalidSeerBaseUrl {
                url: base_url.to_owned(),
                reason: e.to_string(),
            })?;

        if url.cannot_be_a_base() {
            return Err(JellyfinError::InvalidSeerBaseUrl {
                url: base_url.to_owned(),
                reason: "it has no path to hang /api/v1 off".to_owned(),
            });
        }

        Ok(Self { client, base_url: url, api_key, region })
    }

    #[must_use]
    pub const fn base_url(&self) -> &Url {
        &self.base_url
    }

    #[must_use]
    pub fn region(&self) -> &str {
        &self.region
    }

    pub(crate) fn get(&self, path: &str) -> RequestBuilder {
        self.authed(self.client.get(self.endpoint(path)))
    }

    pub(crate) fn get_query(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> RequestBuilder {
        let mut url = self.endpoint(path);
        url.set_query(Some(&encode_query(params)));

        self.authed(self.client.get(url))
    }

    pub(crate) fn post(&self, path: &str) -> RequestBuilder {
        self.authed(self.client.post(self.endpoint(path)))
    }

    fn endpoint(&self, path: &str) -> Url {
        let mut url = self.base_url.clone();

        if let Ok(mut segments) = url.path_segments_mut() {
            segments.extend(["api", "v1"].into_iter().chain(path.split('/')));
        }

        url
    }

    fn authed(&self, builder: RequestBuilder) -> RequestBuilder {
        builder
            .header("X-Api-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
    }
}
