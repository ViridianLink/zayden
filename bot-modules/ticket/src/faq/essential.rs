use reqwest::Client;
use tracing::debug;

use crate::wiki::{self, PageListItem, WikiConfig};

const ESSENTIAL: [&str; 2] = ["faq", "troubleshooting"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Essential {
    pub title: String,
    pub path: String,
}

pub(crate) async fn pages(client: &Client, config: &WikiConfig) -> Vec<Essential> {
    match wiki::list(client, config).await {
        Ok(listed) => select(&listed),
        Err(e) => {
            debug!(error = ?e, "could not list wiki pages for the standing links");
            Vec::new()
        },
    }
}

#[must_use]
pub fn select(listed: &[PageListItem]) -> Vec<Essential> {
    ESSENTIAL.iter().filter_map(|slug| pick(listed, slug)).collect()
}

fn pick(listed: &[PageListItem], slug: &str) -> Option<Essential> {
    listed
        .iter()
        .filter(|page| matches(page, slug))
        .min_by_key(|page| (page.path.matches('/').count(), page.path.len()))
        .map(|page| Essential {
            title: title(page).unwrap_or_else(|| default_title(slug)),
            path: page.path.clone(),
        })
}

fn matches(page: &PageListItem, slug: &str) -> bool {
    let leaf = page.path.rsplit('/').next().unwrap_or(&page.path);

    normalize(leaf) == slug || title(page).is_some_and(|t| normalize(&t) == slug)
}

fn title(page: &PageListItem) -> Option<String> {
    page.title.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned)
}

fn default_title(slug: &str) -> String {
    match slug {
        "faq" => String::from("FAQ"),
        slug => crate::to_title_case(slug),
    }
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
