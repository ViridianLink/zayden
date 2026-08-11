use reqwest::Client;
use serde::Deserialize;

use crate::error::{PalworldError, Result};

const MAX_SAVE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PLAYER_SAVE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ERROR_BODY_CHARS: usize = 400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePlayerSave {
    pub stem: String,
    pub modified: i64,
    pub is_storage: bool,
}

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

async fn panel_ok(resp: reqwest::Response, what: &str) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }

    let body = resp.text().await.unwrap_or_default();
    let detail = body.trim().chars().take(MAX_ERROR_BODY_CHARS).collect::<String>();

    Err(PalworldError::Pelican(if detail.is_empty() {
        format!("{what}: panel returned HTTP {status}")
    } else {
        format!("{what}: panel returned HTTP {status}: {detail}")
    }))
}

pub fn parse_modified_at(raw: &str) -> Result<i64> {
    let ts: jiff::Timestamp = raw.parse().map_err(|e| {
        PalworldError::Pelican(format!("bad modified_at timestamp: {e}"))
    })?;
    Ok(ts.as_second())
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

    async fn list(&self, directory: &str) -> Result<ListResponse> {
        let resp = self
            .client
            .get(self.endpoint("files/list"))
            .query(&[("directory", directory)])
            .bearer_auth(&self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;

        Ok(panel_ok(resp, &format!("listing {directory}")).await?.json().await?)
    }

    pub async fn level_modified(&self) -> Result<i64> {
        let resp = self.list(&self.save_path).await?;

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

        parse_modified_at(modified)
    }

    pub async fn download_level(&self) -> Result<Vec<u8>> {
        let file = format!("{}/Level.sav", self.save_path.trim_end_matches('/'));
        self.download(&file, MAX_SAVE_BYTES).await
    }

    pub async fn list_players(&self) -> Result<Vec<RemotePlayerSave>> {
        let resp = self.list(&self.players_path()).await?;

        Ok(resp
            .data
            .into_iter()
            .filter_map(|f| {
                let stem = f.attributes.name.strip_suffix(".sav")?;
                let modified = f
                    .attributes
                    .modified_at
                    .as_deref()
                    .and_then(|raw| parse_modified_at(raw).ok())
                    .unwrap_or(0);
                Some(RemotePlayerSave {
                    is_storage: stem.ends_with("_dps"),
                    stem: stem.to_string(),
                    modified,
                })
            })
            .collect())
    }

    pub async fn download_player(&self, stem: &str) -> Result<Vec<u8>> {
        let file = format!("{}/{stem}.sav", self.players_path());
        self.download(&file, MAX_PLAYER_SAVE_BYTES).await
    }

    fn players_path(&self) -> String {
        format!("{}/Players", self.save_path.trim_end_matches('/'))
    }

    async fn download(&self, file: &str, max_bytes: u64) -> Result<Vec<u8>> {
        let resp = self
            .client
            .get(self.endpoint("files/download"))
            .query(&[("file", file)])
            .bearer_auth(&self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;

        let signed: SignedUrlResponse =
            panel_ok(resp, &format!("signing download of {file}"))
                .await?
                .json()
                .await?;

        let resp = self.client.get(&signed.attributes.url).send().await?;
        let resp = panel_ok(resp, &format!("downloading {file}")).await?;

        let too_large = |len: u64| {
            PalworldError::Pelican(format!("remote save too large: {len} bytes"))
        };

        if let Some(len) = resp.content_length()
            && len > max_bytes
        {
            return Err(too_large(len));
        }

        let bytes = resp.bytes().await?;
        if bytes.len() as u64 > max_bytes {
            return Err(too_large(bytes.len() as u64));
        }
        Ok(bytes.to_vec())
    }
}
