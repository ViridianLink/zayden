use std::sync::Arc;

use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::error::{MarathonError, Result};

const GAME: &str = "marathon";
const SEED_MAP: &str = "perimeter";
const MAP_DATA_MARKER: &str = "window.mapData = ";
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                                   (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

struct Manifest {
    taxonomy: Arc<Value>,
    roster: Vec<(String, i64)>,
}

pub struct MapGenieDoc {
    pub taxonomy: Arc<Value>,
    pub data: Value,
}

pub struct MapGenie {
    client: Client,
    manifest: Mutex<Option<Arc<Manifest>>>,
}

impl MapGenie {
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self { client, manifest: Mutex::new(None) }
    }

    pub async fn slugs(&self) -> Result<Vec<String>> {
        Ok(self.manifest().await?.roster.iter().map(|(s, _)| s.clone()).collect())
    }

    pub async fn map(&self, slug: &str) -> Result<MapGenieDoc> {
        let manifest = self.manifest().await?;
        let id = manifest
            .roster
            .iter()
            .find(|(s, _)| s == slug)
            .map(|(_, id)| *id)
            .ok_or_else(|| MarathonError::NotFound {
                entity: "mapgenie map",
                query: slug.to_string(),
            })?;
        let data = self.map_data(id).await?;
        Ok(MapGenieDoc { taxonomy: Arc::clone(&manifest.taxonomy), data })
    }

    async fn manifest(&self) -> Result<Arc<Manifest>> {
        let cached = self.manifest.lock().await.clone();
        if let Some(manifest) = cached {
            return Ok(manifest);
        }

        let html =
            self.get(&format!("https://mapgenie.io/{GAME}/maps/{SEED_MAP}")).await?;
        let taxonomy = extract_map_data(&html)?;
        let seed_id = taxonomy
            .get("map")
            .and_then(|m| m.get("id"))
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                MarathonError::Parse("mapgenie window.mapData missing map.id".into())
            })?;

        let roster = roster_from_data(&self.map_data(seed_id).await?);
        if roster.is_empty() {
            return Err(MarathonError::Parse("mapgenie roster is empty".into()));
        }

        let manifest = Arc::new(Manifest { taxonomy: Arc::new(taxonomy), roster });
        *self.manifest.lock().await = Some(Arc::clone(&manifest));
        Ok(manifest)
    }

    async fn map_data(&self, id: i64) -> Result<Value> {
        let text =
            self.get(&format!("https://mapgenie.io/api/v1/maps/{id}/data")).await?;
        serde_json::from_str(&text).map_err(|e| MarathonError::Parse(e.to_string()))
    }

    async fn get(&self, url: &str) -> Result<String> {
        Ok(self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
            .header(reqwest::header::ACCEPT, "text/html,application/json")
            .send()
            .await?
            .text()
            .await?)
    }
}

fn roster_from_data(data: &Value) -> Vec<(String, i64)> {
    data.get("maps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let slug = m.get("slug").and_then(Value::as_str)?.to_string();
            let id = m.get("id").and_then(Value::as_i64)?;
            Some((slug, id))
        })
        .collect()
}

fn extract_map_data(html: &str) -> Result<Value> {
    let (_, rest) = html.split_once(MAP_DATA_MARKER).ok_or_else(|| {
        MarathonError::Parse("mapgenie page has no window.mapData".into())
    })?;
    let json = balanced_object(rest)?;
    serde_json::from_str(json).map_err(|e| MarathonError::Parse(e.to_string()))
}

fn balanced_object(s: &str) -> Result<&str> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;

    for (i, b) in s.bytes().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }

        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return s.get(..=i).ok_or_else(|| {
                        MarathonError::Parse("mapgenie json slicing error".into())
                    });
                }
            },
            _ => {},
        }
    }

    Err(MarathonError::Parse("mapgenie window.mapData is unbalanced".into()))
}
