use reqwest::Client;
use reqwest::header::CONTENT_TYPE;
use scraper::{Html, Selector};
use tracing::debug;
use url::Url;

const MAX_PAGES: usize = 3;
const MAX_BODY_BYTES: usize = 512 * 1024;
const EXCERPT_LIMIT: usize = 2_000;

const SCHEMES: [&str; 2] = ["https://", "http://"];
const TRAILING: [char; 11] =
    ['.', ',', ';', ':', '!', '?', ')', ']', '>', '"', '\''];

// Markdown, Discord's angle-bracket link suppression and table pipes all end a
// URL without being whitespace.
const BOUNDARY: [char; 4] = ['<', '>', '|', '`'];

const READABLE: &str = "p, li, h1, h2, h3, h4, h5, h6, pre, td, th, dt, dd, \
                        blockquote, figcaption";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedPage {
    pub url: String,
    pub text: String,
}

#[must_use]
pub fn urls(content: &str, limit: usize) -> Vec<Url> {
    let mut found: Vec<Url> = Vec::new();
    let mut rest = content;

    while let Some(start) = SCHEMES.iter().filter_map(|s| rest.find(s)).min() {
        let Some(tail) = rest.get(start..) else { break };

        let end = tail
            .find(|c: char| c.is_whitespace() || BOUNDARY.contains(&c))
            .unwrap_or(tail.len());

        let Some((raw, after)) = tail.split_at_checked(end) else { break };
        rest = after;

        let Ok(url) = Url::parse(raw.trim_end_matches(TRAILING)) else {
            continue;
        };

        if url.scheme() != "http" && url.scheme() != "https" {
            continue;
        }

        if !found.contains(&url) {
            found.push(url);
        }

        if found.len() == limit {
            break;
        }
    }

    found
}

pub async fn pages(client: &Client, content: &str) -> Vec<LinkedPage> {
    let mut pages = Vec::new();

    for url in urls(content, MAX_PAGES) {
        match fetch(client, &url).await {
            Ok(Some(text)) => {
                pages.push(LinkedPage { url: url.to_string(), text });
            },
            Ok(None) => debug!(%url, "linked page held no readable text"),
            Err(e) => debug!(%url, "could not read linked page: {e}"),
        }
    }

    pages
}

async fn fetch(client: &Client, url: &Url) -> reqwest::Result<Option<String>> {
    let mut response = client.get(url.clone()).send().await?.error_for_status()?;

    let html = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            let kind = value.split(';').next().unwrap_or(value).trim();

            match kind {
                "text/html" | "application/xhtml+xml" => Some(true),
                _ if kind.starts_with("text/") => Some(false),
                "application/json" | "application/xml" => Some(false),
                _ => None,
            }
        });

    let Some(Some(html)) = html else { return Ok(None) };

    let mut body = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        let room = MAX_BODY_BYTES.saturating_sub(body.len());

        if room == 0 {
            break;
        }

        body.extend_from_slice(chunk.get(..room.min(chunk.len())).unwrap_or(&chunk));
    }

    let body = String::from_utf8_lossy(&body);
    let text = if html { readable(&body) } else { body.into_owned() };

    Ok(excerpt(&text, EXCERPT_LIMIT))
}

#[must_use]
pub fn readable(body: &str) -> String {
    let Ok(selector) = Selector::parse(READABLE) else {
        return String::new();
    };

    Html::parse_document(body)
        .select(&selector)
        .map(|element| element.text().collect::<String>().trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn excerpt(text: &str, limit: usize) -> Option<String> {
    let mut kept = String::with_capacity(limit.min(text.len()));

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if kept.chars().count() + line.chars().count() > limit {
            break;
        }

        if !kept.is_empty() {
            kept.push('\n');
        }

        kept.push_str(line);
    }

    if kept.is_empty() { None } else { Some(kept) }
}
