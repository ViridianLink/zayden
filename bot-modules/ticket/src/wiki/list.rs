#![expect(
    unused_qualifications,
    reason = "cynic's QueryFragment derive emits fully qualified paths spanned to \
              the field declarations they were generated from"
)]

use cynic::{QueryBuilder, QueryFragment, QueryVariables};
use reqwest::Client;

use crate::wiki::{WikiConfig, WikiError, graphql, schema};

#[derive(QueryVariables)]
struct ListVariables<'a> {
    locale: Option<&'a str>,
}

#[derive(QueryFragment)]
#[cynic(graphql_type = "Query", variables = "ListVariables")]
struct ListPages {
    pages: Option<PageQuery>,
}

#[derive(QueryFragment)]
#[cynic(variables = "ListVariables")]
struct PageQuery {
    #[arguments(locale: $locale)]
    list: Vec<PageListItem>,
}

#[derive(QueryFragment, Debug, Clone)]
pub struct PageListItem {
    pub id: i32,
    pub path: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_published: bool,
}

pub async fn list(
    client: &Client,
    config: &WikiConfig,
) -> Result<Vec<PageListItem>, WikiError> {
    let operation =
        ListPages::build(ListVariables { locale: Some(config.locale()) });

    let data = graphql::run(client, config, &operation).await?;

    Ok(data
        .pages
        .map(|pages| {
            pages.list.into_iter().filter(|page| page.is_published).collect()
        })
        .unwrap_or_default())
}
