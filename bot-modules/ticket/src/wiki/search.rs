use reqwest::Client;
use serde::Deserialize;

use crate::wiki::{WikiConfig, WikiError, graphql};

const SEARCH_QUERY: &str = r"
    query SearchPages($query: String!, $locale: String!) {
        pages {
            search(query: $query, locale: $locale) {
                results {
                    title
                    description
                    path
                }
            }
        }
    }
";

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub description: String,
    pub path: String,
}

#[derive(Deserialize)]
struct SearchData {
    pages: PagesSearch,
}

#[derive(Deserialize)]
struct PagesSearch {
    search: SearchResults,
}

#[derive(Deserialize)]
struct SearchResults {
    results: Vec<SearchResult>,
}

pub async fn search(
    client: &Client,
    config: &WikiConfig,
    query: &str,
) -> Result<Vec<SearchResult>, WikiError> {
    let body = serde_json::json!({
        "query": SEARCH_QUERY,
        "variables": { "query": query, "locale": config.locale() },
    });

    let data: SearchData = graphql::query(client, config, &body).await?;

    Ok(data.pages.search.results)
}
