pub mod items;
pub mod libraries;
pub mod model;
pub mod quick_connect;
pub mod users;

use reqwest::{Client, RequestBuilder};

use crate::transport::http::trim_base_url;

pub const SERVICE: &str = "Jellyfin";

#[must_use]
pub fn device_auth_header(device_id: &str, client_version: &str) -> String {
    format!(
        r#"MediaBrowser Client="Zayden", Device="Discord", DeviceId="{device_id}", Version="{client_version}""#
    )
}

#[derive(Debug, Clone)]
pub struct JellyfinClient {
    client: Client,
    base_url: String,
    api_key: String,
    movie_library_id: String,
    show_library_id: String,
}

impl JellyfinClient {
    #[must_use]
    pub fn new(
        client: Client,
        base_url: String,
        api_key: String,
        movie_library_id: String,
        show_library_id: String,
    ) -> Self {
        Self {
            client,
            base_url: trim_base_url(base_url),
            api_key,
            movie_library_id,
            show_library_id,
        }
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    #[must_use]
    pub fn movie_library_id(&self) -> &str {
        &self.movie_library_id
    }

    #[must_use]
    pub fn show_library_id(&self) -> &str {
        &self.show_library_id
    }

    #[must_use]
    pub fn item_url(&self, item_id: &str) -> String {
        format!("{}/web/#/details?id={item_id}", self.base_url)
    }

    #[must_use]
    pub fn quick_connect_url(&self) -> String {
        format!("{}/web/#/quickconnect", self.base_url)
    }

    pub(crate) fn endpoint(&self, path: &str) -> String {
        format!("{}/{path}", self.base_url)
    }

    pub(crate) fn get(&self, path: &str) -> RequestBuilder {
        self.authed(self.client.get(self.endpoint(path)))
    }

    pub(crate) fn post(&self, path: &str) -> RequestBuilder {
        self.authed(self.client.post(self.endpoint(path)))
    }

    pub(crate) fn delete(&self, path: &str) -> RequestBuilder {
        self.authed(self.client.delete(self.endpoint(path)))
    }

    pub(crate) fn authed(&self, builder: RequestBuilder) -> RequestBuilder {
        builder
            .header(
                reqwest::header::AUTHORIZATION,
                format!(r#"MediaBrowser Token="{}""#, self.api_key),
            )
            .header(reqwest::header::ACCEPT, "application/json")
    }

    pub(crate) fn device(&self, path: &str, device_id: &str) -> RequestBuilder {
        self.client.post(self.endpoint(path)).header(
            reqwest::header::AUTHORIZATION,
            device_auth_header(device_id, env!("CARGO_PKG_VERSION")),
        )
    }
}
