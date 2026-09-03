use std::collections::HashSet;

use reqwest::Client;
use tracing::warn;

use crate::wiki::{SearchResult, WikiConfig, search};

const RESULTS_PER_KEYWORD: usize = 3;

pub(crate) async fn search_keywords(
    client: &Client,
    config: &WikiConfig,
    keywords: &[String],
) -> Vec<SearchResult> {
    let mut seen = HashSet::new();
    let mut aggregated = Vec::new();

    for keyword in keywords {
        let pages = match search(client, config, keyword).await {
            Ok(pages) => pages,
            Err(e) => {
                warn!(error = ?e, keyword, "wiki search failed for keyword");
                continue;
            },
        };

        aggregated.extend(
            pages
                .into_iter()
                .take(RESULTS_PER_KEYWORD)
                .filter(|page| seen.insert(page.path.clone())),
        );
    }

    aggregated.truncate(config.max_results());
    aggregated
}
