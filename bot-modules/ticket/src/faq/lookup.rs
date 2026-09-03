use std::collections::HashSet;

use reqwest::Client;
use serenity::all::GuildId;
use sqlx::PgPool;
use tracing::warn;
use zayden_core::as_i64;

use crate::faq::article::FaqArticle;
use crate::faq::hit::FaqHit;
use crate::wiki::{WikiConfig, search};

const RESULTS_PER_KEYWORD: i64 = 3;

pub(crate) async fn search_keywords(
    pool: &PgPool,
    guild_id: GuildId,
    client: &Client,
    config: &WikiConfig,
    keywords: &[String],
) -> Vec<FaqHit> {
    let mut seen = HashSet::new();
    let mut aggregated = Vec::new();

    for keyword in keywords {
        match FaqArticle::search(
            pool,
            as_i64(guild_id.get()),
            keyword,
            RESULTS_PER_KEYWORD,
        )
        .await
        {
            Ok(articles) => aggregated.extend(
                articles
                    .iter()
                    .map(FaqHit::from)
                    .filter(|hit| seen.insert(hit.path.clone())),
            ),
            Err(e) => warn!(error = ?e, keyword, "faq article search failed"),
        }
    }

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
                .take(usize::try_from(RESULTS_PER_KEYWORD).unwrap_or(3))
                .map(FaqHit::from)
                .filter(|hit| seen.insert(hit.path.clone())),
        );
    }

    aggregated.truncate(config.max_results());
    aggregated
}
