use std::collections::HashSet;

use jellyfin::JellyfinError;
use jellyfin::index::LibraryItemRow;
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::Client;
use sqlx::PgPool;

use crate::error::{Result, WatchError};

const MAX_FEED_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct DiaryEntry {
    pub title: String,
    pub year: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub rating: Option<f32>,
    pub watched_date: Option<String>,
    pub rewatch: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GapReport {
    pub total: usize,
    pub matched_by_id: usize,
    pub matched_by_title: usize,
    pub unmatched: usize,
    pub missing: Vec<DiaryEntry>,
}

#[must_use]
pub fn feed_url(username: &str) -> String {
    format!("https://letterboxd.com/{}/rss/", username.trim().trim_matches('/'))
}

pub async fn fetch(client: &Client, username: &str) -> Result<Vec<DiaryEntry>> {
    let response = client
        .get(feed_url(username))
        .header(reqwest::header::ACCEPT, "application/rss+xml")
        .send()
        .await
        .map_err(|e| {
            WatchError::Jellyfin(JellyfinError::LetterboxdParse(e.to_string()))
        })?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(JellyfinError::NoLetterboxdFeed(username.to_owned()).into());
    }

    let body = response.text().await.map_err(|e| {
        WatchError::Jellyfin(JellyfinError::LetterboxdParse(e.to_string()))
    })?;

    if body.len() > MAX_FEED_BYTES {
        return Err(JellyfinError::LetterboxdParse(
            "feed is implausibly large".to_owned(),
        )
        .into());
    }

    parse(&body)
}

pub fn parse(xml: &str) -> Result<Vec<DiaryEntry>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut entries = Vec::new();
    let mut current: Option<DiaryEntry> = None;
    let mut field = String::new();

    loop {
        let event = reader.read_event().map_err(|e| {
            WatchError::Jellyfin(JellyfinError::LetterboxdParse(e.to_string()))
        })?;

        match event {
            Event::Start(tag) => {
                let name = tag.name().as_ref().to_owned();

                if name == "item" {
                    current = Some(DiaryEntry {
                        title: String::new(),
                        year: None,
                        tmdb_id: None,
                        rating: None,
                        watched_date: None,
                        rewatch: false,
                    });
                }
                field = name;
            },
            Event::Text(text) => {
                let Some(entry) = current.as_mut() else {
                    continue;
                };
                let value = text.xml10_content().into_owned();

                match field.as_str() {
                    "letterboxd:filmTitle" => entry.title = value,
                    "letterboxd:filmYear" => entry.year = value.parse().ok(),
                    "tmdb:movieId" => entry.tmdb_id = value.parse().ok(),
                    "letterboxd:memberRating" => entry.rating = value.parse().ok(),
                    "letterboxd:watchedDate" => entry.watched_date = Some(value),
                    "letterboxd:rewatch" => entry.rewatch = value == "Yes",
                    _ => {},
                }
            },
            Event::End(tag) => {
                if tag.name().as_ref() == "item"
                    && let Some(entry) = current.take()
                    && !entry.title.is_empty()
                {
                    entries.push(entry);
                }
                field.clear();
            },
            Event::Eof => break,
            Event::Empty(_)
            | Event::CData(_)
            | Event::Comment(_)
            | Event::Decl(_)
            | Event::PI(_)
            | Event::DocType(_)
            | Event::GeneralRef(_) => {},
        }
    }

    Ok(entries)
}

pub async fn cross_reference(
    pool: &PgPool,
    entries: &[DiaryEntry],
    min_rating: f32,
    limit: usize,
) -> Result<GapReport> {
    let owned: HashSet<i32> =
        LibraryItemRow::tmdb_ids(pool, "Movie").await?.into_iter().collect();

    let mut report = GapReport { total: entries.len(), ..Default::default() };
    let mut missing = Vec::new();

    for entry in entries {
        match entry.tmdb_id {
            Some(tmdb_id) => {
                report.matched_by_id += 1;
                if !owned.contains(&tmdb_id) {
                    missing.push(entry.clone());
                }
            },
            None => {
                // No id in the feed: fall back to title, and say so in the
                // report rather than presenting it as an exact match.
                let found =
                    LibraryItemRow::search(pool, &entry.title, Some("Movie"), 1)
                        .await?;

                if found.is_empty() {
                    report.unmatched += 1;
                    missing.push(entry.clone());
                } else {
                    report.matched_by_title += 1;
                }
            },
        }
    }

    missing.retain(|e| e.rating.is_none_or(|r| r >= min_rating));
    missing
        .sort_by(|a, b| b.rating.unwrap_or(0.0).total_cmp(&a.rating.unwrap_or(0.0)));
    missing.truncate(limit);

    report.missing = missing;
    Ok(report)
}
