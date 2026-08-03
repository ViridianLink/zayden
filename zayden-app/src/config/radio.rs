use std::collections::HashSet;
use std::sync::Arc;

use serde::Deserialize;
use tracing::warn;

#[derive(Debug, Clone, Deserialize)]
pub struct RadioStation {
    pub id: String,
    pub name: String,
    pub stream_url: String,
    pub genre: Option<String>,
    pub homepage: Option<String>,
    pub logo_url: Option<String>,
}

impl RadioStation {
    #[must_use]
    pub fn display_url(&self) -> &str {
        self.homepage.as_deref().unwrap_or(&self.stream_url)
    }

    #[must_use]
    pub fn matches(&self, needle: &str) -> bool {
        let needle = needle.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }

        self.name.to_lowercase().contains(&needle)
            || self.id.to_lowercase().contains(&needle)
            || self
                .genre
                .as_ref()
                .is_some_and(|g| g.to_lowercase().contains(&needle))
    }
}

fn is_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[must_use]
pub fn validate_all(stations: Vec<RadioStation>) -> Arc<[RadioStation]> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut valid: Vec<RadioStation> = Vec::with_capacity(stations.len());

    for station in stations {
        if station.id.trim().is_empty() || station.name.trim().is_empty() {
            warn!(
                id = %station.id,
                "radio station dropped: id and name must both be non-empty"
            );
            continue;
        }

        if !is_http_url(&station.stream_url) {
            warn!(
                id = %station.id,
                "radio station dropped: stream_url must be an http(s) URL"
            );
            continue;
        }

        if !seen.insert(station.id.clone()) {
            warn!(id = %station.id, "radio station dropped: duplicate id");
            continue;
        }

        valid.push(station);
    }

    valid.into()
}

#[must_use]
pub fn find<'a>(stations: &'a [RadioStation], id: &str) -> Option<&'a RadioStation> {
    stations.iter().find(|station| station.id == id)
}
