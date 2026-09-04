#![expect(
    unused_qualifications,
    reason = "cynic's QueryFragment derive emits fully qualified paths spanned to \
              the field declarations they were generated from"
)]

use cynic::{QueryBuilder, QueryFragment, QueryVariables};
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::debug;

use crate::wiki::{WikiConfig, WikiError, graphql, schema};

#[derive(QueryVariables)]
struct IdVariables {
    id: i32,
}

#[derive(QueryFragment)]
#[cynic(graphql_type = "Query", variables = "IdVariables")]
struct GetPageById {
    pages: Option<PageByIdQuery>,
}

#[derive(QueryFragment)]
#[cynic(graphql_type = "PageQuery", variables = "IdVariables")]
struct PageByIdQuery {
    #[arguments(id: $id)]
    single: Option<Page>,
}

#[derive(QueryVariables)]
struct PageVariables<'a> {
    path: &'a str,
    locale: &'a str,
}

#[derive(QueryFragment)]
#[cynic(graphql_type = "Query", variables = "PageVariables")]
struct GetPage {
    pages: Option<PageQuery>,
}

#[derive(QueryFragment)]
#[cynic(variables = "PageVariables")]
struct PageQuery {
    #[arguments(path: $path, locale: $locale)]
    single_by_path: Option<Page>,
}

#[derive(QueryFragment, Debug, Clone)]
pub struct Page {
    pub title: String,
    pub path: String,
    pub content: String,
}

pub async fn page(
    client: &Client,
    config: &WikiConfig,
    path: &str,
) -> Result<Page, WikiError> {
    match page_via_graphql(client, config, path).await {
        Err(WikiError::PageForbidden) => {
            debug!(path, "graphql page source forbidden; trying the source view");
            page_via_source_view(client, config, path).await
        },
        result => result,
    }
}

pub async fn page_by_id(
    client: &Client,
    config: &WikiConfig,
    id: i32,
) -> Result<Page, WikiError> {
    let operation = GetPageById::build(IdVariables { id });

    let data = graphql::run(client, config, &operation).await?;

    data.pages
        .and_then(|pages| pages.single)
        .ok_or_else(|| WikiError::PageNotFound(id.to_string()))
}

async fn page_via_graphql(
    client: &Client,
    config: &WikiConfig,
    path: &str,
) -> Result<Page, WikiError> {
    let operation = GetPage::build(PageVariables { path, locale: config.locale() });

    let data = graphql::run(client, config, &operation).await?;

    data.pages
        .and_then(|pages| pages.single_by_path)
        .ok_or_else(|| WikiError::PageNotFound(path.to_owned()))
}

async fn page_via_source_view(
    client: &Client,
    config: &WikiConfig,
    path: &str,
) -> Result<Page, WikiError> {
    let mut request = client.get(config.source_url(path)?);

    if let Some(key) = config.api_key() {
        request = request.bearer_auth(key);
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(WikiError::SourceView(response.status().as_u16()));
    }

    let body = response.text().await?;

    parse_source_view(&body)
        .map(|content| Page {
            title: page_title(&body).unwrap_or_else(|| path.to_owned()),
            path: path.to_owned(),
            content,
        })
        .ok_or(WikiError::PageForbidden)
}

fn parse_source_view(body: &str) -> Option<String> {
    let selector = Selector::parse("code[v-pre]").ok()?;
    let document = Html::parse_document(body);

    let text = document.select(&selector).next()?.text().collect::<String>();

    if text.trim().is_empty() { None } else { Some(text) }
}

fn page_title(body: &str) -> Option<String> {
    let selector = Selector::parse("title").ok()?;
    let document = Html::parse_document(body);
    let raw = document.select(&selector).next()?.text().collect::<String>();

    // Wiki.js titles its pages "<page title> | <site title>".
    let title = raw.split('|').next().unwrap_or(&raw).trim();

    if title.is_empty() { None } else { Some(title.to_owned()) }
}
