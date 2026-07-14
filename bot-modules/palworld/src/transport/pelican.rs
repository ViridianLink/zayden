use reqwest::Client;
use serde::Deserialize;

use crate::error::{PalworldError, Result};

const MAX_SAVE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Pelican {
    client: Client,
    base_url: String,
    api_key: String,
    server_id: String,
    save_path: String,
}

#[derive(Deserialize)]
struct ListResponse {
    data: Vec<FileObject>,
}

#[derive(Deserialize)]
struct FileObject {
    attributes: FileAttributes,
}

#[derive(Deserialize)]
struct FileAttributes {
    name: String,
    #[serde(default)]
    modified_at: Option<String>,
}

#[derive(Deserialize)]
struct SignedUrlResponse {
    attributes: SignedUrlAttributes,
}

#[derive(Deserialize)]
struct SignedUrlAttributes {
    url: String,
}

impl Pelican {
    #[must_use]
    pub fn new(
        client: Client,
        base_url: String,
        api_key: String,
        server_id: String,
        save_path: String,
    ) -> Self {
        let mut base_url = base_url;
        while base_url.ends_with('/') {
            base_url.pop();
        }
        Self { client, base_url, api_key, server_id, save_path }
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}/api/client/servers/{}/{path}", self.base_url, self.server_id)
    }

    pub async fn level_modified(&self) -> Result<i64> {
        let resp: ListResponse = self
            .client
            .get(self.endpoint("files/list"))
            .query(&[("directory", self.save_path.as_str())])
            .bearer_auth(&self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let modified = resp
            .data
            .iter()
            .find(|f| f.attributes.name == "Level.sav")
            .and_then(|f| f.attributes.modified_at.as_deref())
            .ok_or_else(|| {
                PalworldError::Pelican(
                    "Level.sav not found in remote save directory".to_string(),
                )
            })?;

        let ts: jiff::Timestamp = modified.parse().map_err(|e| {
            PalworldError::Pelican(format!("bad modified_at timestamp: {e}"))
        })?;
        Ok(ts.as_second())
    }

    pub async fn download_level(&self) -> Result<Vec<u8>> {
        let file = format!("{}/Level.sav", self.save_path.trim_end_matches('/'));

        let signed: SignedUrlResponse = self
            .client
            .get(self.endpoint("files/download"))
            .query(&[("file", file.as_str())])
            .bearer_auth(&self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let resp = self
            .client
            .get(&signed.attributes.url)
            .send()
            .await?
            .error_for_status()?;

        if let Some(len) = resp.content_length()
            && len > MAX_SAVE_BYTES
        {
            return Err(PalworldError::Pelican(format!(
                "remote save too large: {len} bytes"
            )));
        }

        let bytes = resp.bytes().await?;
        if bytes.len() as u64 > MAX_SAVE_BYTES {
            return Err(PalworldError::Pelican(format!(
                "remote save too large: {} bytes",
                bytes.len()
            )));
        }
        Ok(bytes.to_vec())
    }
}
