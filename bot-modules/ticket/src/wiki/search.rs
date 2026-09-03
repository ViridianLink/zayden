#![expect(
    unused_qualifications,
    reason = "cynic's QueryFragment derive emits fully qualified paths spanned to \
              the field declarations they were generated from"
)]

use cynic::{QueryBuilder, QueryFragment, QueryVariables};
use reqwest::Client;

use crate::wiki::{WikiConfig, WikiError, graphql, schema};

#[derive(QueryVariables)]
struct SearchVariables<'a> {
    query: &'a str,
    locale: Option<&'a str>,
}

#[derive(QueryFragment)]
#[cynic(graphql_type = "Query", variables = "SearchVariables")]
struct SearchPages {
    pages: Option<PageQuery>,
}

#[derive(QueryFragment)]
#[cynic(variables = "SearchVariables")]
struct PageQuery {
    #[arguments(query: $query, locale: $locale)]
    search: PageSearchResponse,
}

#[derive(QueryFragment)]
struct PageSearchResponse {
    results: Vec<Option<SearchResult>>,
}

#[derive(QueryFragment, Debug, Clone)]
#[cynic(graphql_type = "PageSearchResult")]
pub struct SearchResult {
    pub title: String,
    pub description: String,
    pub path: String,
}

pub async fn search(
    client: &Client,
    config: &WikiConfig,
    query: &str,
) -> Result<Vec<SearchResult>, WikiError> {
    let operation =
        SearchPages::build(SearchVariables { query, locale: Some(config.locale()) });

    let data = graphql::run(client, config, &operation).await?;

    Ok(data
        .pages
        .map(|pages| pages.search.results.into_iter().flatten().collect())
        .unwrap_or_default())
}
