use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::Client;

use crate::discovery::gaps::Candidate;
use crate::error::{JellyfinError, Result};

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

impl DiaryEntry {
    #[must_use]
    pub fn candidate(&self) -> Candidate {
        Candidate {
            title: self.title.clone(),
            year: self.year,
            tmdb_id: self.tmdb_id,
            rating: self.rating,
        }
    }
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
        .map_err(|e| JellyfinError::LetterboxdParse(e.to_string()))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(JellyfinError::NoLetterboxdFeed(username.to_owned()));
    }

    let body = response
        .text()
        .await
        .map_err(|e| JellyfinError::LetterboxdParse(e.to_string()))?;

    if body.len() > MAX_FEED_BYTES {
        return Err(JellyfinError::LetterboxdParse(
            "feed is implausibly large".to_owned(),
        ));
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
        let event = reader
            .read_event()
            .map_err(|e| JellyfinError::LetterboxdParse(e.to_string()))?;

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
