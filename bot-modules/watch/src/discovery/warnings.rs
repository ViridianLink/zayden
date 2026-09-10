use jellyfin::JellyfinError;
use reqwest::Client;
use serde::Deserialize;

use crate::error::{Result, WatchError};

const API_ROOT: &str = "https://www.doesthedogdie.com";

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    #[serde(default = "Vec::new")]
    items: Vec<SearchItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchItem {
    id: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct MediaResponse {
    #[serde(rename = "topicItemStats", default = "Vec::new")]
    topics: Vec<TopicStat>,
}

#[derive(Debug, Clone, Deserialize)]
struct TopicStat {
    #[serde(default)]
    topic: Option<Topic>,
    #[serde(rename = "yesSum", default)]
    yes: i64,
    #[serde(rename = "noSum", default)]
    no: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct Topic {
    #[serde(rename = "doesName", default)]
    does_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Mild,
    Moderate,
    Severe,
}

impl Severity {
    #[must_use]
    pub const fn emoji(self) -> &'static str {
        match self {
            Self::Mild => "🟢",
            Self::Moderate => "🟡",
            Self::Severe => "🔴",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub topic: String,
    pub severity: Severity,
    pub yes: i64,
    pub no: i64,
}

const fn severity(yes: i64, no: i64) -> Option<Severity> {
    let total = yes + no;
    if total < 3 || yes == 0 {
        return None;
    }

    let share = (yes * 100) / total;
    Some(match share {
        0..=39 => Severity::Mild,
        40..=74 => Severity::Moderate,
        _ => Severity::Severe,
    })
}

pub async fn lookup(
    client: &Client,
    api_key: Option<&str>,
    title: &str,
) -> Result<Vec<Warning>> {
    let key = api_key.ok_or(JellyfinError::WarningsUnavailable)?;

    let search: SearchResponse = client
        .get(format!("{API_ROOT}/dddsearch"))
        .query(&[("q", title)])
        .header("X-API-KEY", key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| api_err(&e))?
        .json()
        .await
        .map_err(|e| api_err(&e))?;

    let best = search
        .items
        .first()
        .ok_or_else(|| JellyfinError::NoSuchTitle(title.to_owned()))?;

    let media: MediaResponse = client
        .get(format!("{API_ROOT}/media/{}", best.id))
        .header("X-API-KEY", key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| api_err(&e))?
        .json()
        .await
        .map_err(|e| api_err(&e))?;

    let mut warnings: Vec<Warning> = media
        .topics
        .iter()
        .filter_map(|stat| {
            let topic = stat.topic.as_ref()?.does_name.clone()?;
            Some(Warning {
                topic,
                severity: severity(stat.yes, stat.no)?,
                yes: stat.yes,
                no: stat.no,
            })
        })
        .collect();

    warnings.sort_by_key(|w| std::cmp::Reverse(w.yes));
    Ok(warnings)
}

fn api_err(e: &reqwest::Error) -> WatchError {
    WatchError::Jellyfin(JellyfinError::Internal(format!(
        "doesthedogdie request failed: {e}"
    )))
}
