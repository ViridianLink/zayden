use std::collections::HashSet;

use futures::future::join_all;
use reqwest::Client;
use serenity::all::GuildId;
use sqlx::PgPool;
use tracing::warn;
use zayden_core::as_i64;

use crate::faq::article::FaqArticle;
use crate::faq::hit::FaqHit;
use crate::wiki::{WikiConfig, search};

const RESULTS_PER_KEYWORD: usize = 3;
const SOLVED_LIMIT: i64 = 4;

pub(crate) struct Findings {
    pub articles: Vec<FaqHit>,
    pub solved: Vec<FaqArticle>,
}

pub(crate) async fn search_keywords(
    pool: &PgPool,
    guild_id: GuildId,
    client: &Client,
    config: &WikiConfig,
    keywords: &[String],
) -> Findings {
    let (articles, solved) = tokio::join!(
        wiki_articles(client, config, keywords),
        solved_tickets(pool, guild_id, keywords),
    );

    Findings { articles, solved }
}

async fn wiki_articles(
    client: &Client,
    config: &WikiConfig,
    keywords: &[String],
) -> Vec<FaqHit> {
    let searches =
        join_all(keywords.iter().map(|keyword| search(client, config, keyword)))
            .await;

    let per_keyword = keywords
        .iter()
        .zip(searches)
        .filter_map(|(keyword, result)| {
            result
                .inspect_err(|e| {
                    warn!(error = ?e, keyword, "wiki search failed for keyword");
                })
                .ok()
        })
        .collect::<Vec<_>>();

    let mut seen = HashSet::new();

    let mut hits = (0..RESULTS_PER_KEYWORD)
        .flat_map(|rank| per_keyword.iter().filter_map(move |pages| pages.get(rank)))
        .cloned()
        .map(FaqHit::from)
        .filter(|hit| seen.insert(hit.path.clone()))
        .collect::<Vec<_>>();

    hits.truncate(config.max_results());
    hits
}

async fn solved_tickets(
    pool: &PgPool,
    guild_id: GuildId,
    keywords: &[String],
) -> Vec<FaqArticle> {
    if keywords.is_empty() {
        return Vec::new();
    }

    FaqArticle::similar(
        pool,
        as_i64(guild_id.get()),
        &keywords.join(" "),
        None,
        SOLVED_LIMIT,
    )
    .await
    .unwrap_or_else(|e| {
        warn!(error = ?e, "solved ticket lookup failed");
        Vec::new()
    })
}
