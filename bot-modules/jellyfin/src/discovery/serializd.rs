use std::time::Duration;

use reqwest::{Client, StatusCode, header};
use serde::Deserialize;

use crate::discovery::gaps::Candidate;
use crate::error::{JellyfinError, Result};
use crate::transport::http::encode_path_segment;

const API_BASE: &str = "https://serializd.onrender.com/api";
const FRONT_PAGE: &str = "https://www.serializd.com";
const APP_ID: &str = "serializd_vercel";

const MAX_PAGES: u32 = 3;
const MAX_PAGE_BYTES: usize = 4 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub struct DiaryEntry {
    pub tmdb_id: i32,
    pub title: String,
    pub year: Option<i32>,
    pub rating: Option<f32>,
    pub logged_date: Option<String>,
    pub rewatch: bool,
}

impl DiaryEntry {
    #[must_use]
    pub fn candidate(&self) -> Candidate {
        Candidate {
            title: self.title.clone(),
            year: self.year,
            tmdb_id: Some(self.tmdb_id),
            rating: self.rating,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiaryPage {
    pub entries: Vec<DiaryEntry>,
    pub total_pages: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPage {
    #[serde(default)]
    reviews: Vec<RawEntry>,
    total_pages: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEntry {
    show_id: Option<i32>,
    show_name: Option<String>,
    show_premiere_date: Option<String>,
    rating: Option<i32>,
    backdate: Option<String>,
    date_added: Option<String>,
    is_rewatched: Option<bool>,
}

impl RawEntry {
    fn into_entry(self) -> Option<DiaryEntry> {
        let tmdb_id = self.show_id.filter(|id| *id > 0)?;
        let title = self
            .show_name
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())?;

        Some(DiaryEntry {
            tmdb_id,
            title,
            year: self
                .show_premiere_date
                .as_deref()
                .and_then(|date| date.get(..4))
                .and_then(|year| year.parse().ok()),
            rating: self.rating.and_then(stars),
            logged_date: self.backdate.or(self.date_added),
            rewatch: self.is_rewatched.unwrap_or_default(),
        })
    }
}

#[must_use]
pub fn diary_url(username: &str, page: u32) -> String {
    format!(
        "{API_BASE}/user/{}/diary?page={page}",
        encode_path_segment(username.trim().trim_matches('/'))
    )
}

#[must_use]
pub fn stars(rating: i32) -> Option<f32> {
    u8::try_from(rating)
        .ok()
        .filter(|out_of_ten| (1..=10).contains(out_of_ten))
        .map(|out_of_ten| f32::from(out_of_ten) / 2.0)
}

pub async fn fetch(client: &Client, username: &str) -> Result<Vec<DiaryEntry>> {
    let mut entries = Vec::new();

    for page in 1..=MAX_PAGES {
        let parsed = parse(&fetch_page(client, username, page).await?)?;
        entries.extend(parsed.entries);

        if page >= parsed.total_pages {
            break;
        }
    }

    Ok(entries)
}

async fn fetch_page(client: &Client, username: &str, page: u32) -> Result<String> {
    let response = client
        .get(diary_url(username, page))
        .header(header::ACCEPT, "application/json")
        .header(header::ORIGIN, FRONT_PAGE)
        .header(header::REFERER, FRONT_PAGE)
        .header("X-Requested-With", APP_ID)
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|e| JellyfinError::SerializdParse(e.to_string()))?;

    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        return Err(JellyfinError::NoSerializdDiary(username.trim().to_owned()));
    }
    if !status.is_success() {
        return Err(JellyfinError::SerializdParse(format!("HTTP {status}")));
    }

    let body = response
        .text()
        .await
        .map_err(|e| JellyfinError::SerializdParse(e.to_string()))?;

    if body.len() > MAX_PAGE_BYTES {
        return Err(JellyfinError::SerializdParse(
            "diary page is implausibly large".to_owned(),
        ));
    }

    Ok(body)
}

pub fn parse(json: &str) -> Result<DiaryPage> {
    let raw: RawPage = serde_json::from_str(json)
        .map_err(|e| JellyfinError::SerializdParse(e.to_string()))?;

    Ok(DiaryPage {
        entries: raw.reviews.into_iter().filter_map(RawEntry::into_entry).collect(),
        total_pages: raw.total_pages.unwrap_or(1),
    })
}
