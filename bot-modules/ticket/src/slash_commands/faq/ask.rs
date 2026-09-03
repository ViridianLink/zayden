use std::collections::{HashMap, HashSet};

use serenity::all::{
    CommandInteraction,
    CreateEmbed,
    EditInteractionResponse,
    GuildId,
    Http,
    ResolvedValue,
};
use tracing::{error, warn};
use zayden_app::state::AppState;
use zayden_core::{as_i64, required_option};

use crate::faq::article::FaqArticle;
use crate::faq::hit::{FaqHit, FaqSource};
use crate::faq::{FaqContext, answer, embeds};
use crate::wiki::{self, Page, WikiConfig};
use crate::{Result, Ticket, TicketError};

impl Ticket {
    pub(super) async fn faq_ask(
        http: &Http,
        interaction: &CommandInteraction,
        app: &AppState,
        mut options: HashMap<&str, ResolvedValue<'_>>,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let query: &str = required_option(&mut options, "query")?;

        let context = FaqContext::load(&app.settings.faq, guild_id)
            .await
            .map_err(|e| TicketError::Internal(e.to_string()))?
            .ok_or(TicketError::FaqNotConfigured)?;

        let limit = context.wiki.max_results();
        let guild = as_i64(guild_id.get());

        let articles = match FaqArticle::search(
            &app.db,
            guild,
            query,
            limit_as_i64(limit),
        )
        .await
        {
            Ok(articles) => articles,
            Err(e) => {
                error!(error = ?e, query, %guild_id, "faq article search failed");
                Vec::new()
            },
        };

        let pages = match wiki::search(&app.http, &context.wiki, query).await {
            Ok(pages) => pages,
            // A wiki outage should not hide articles this server wrote itself.
            Err(e) if !articles.is_empty() => {
                warn!(error = ?e, query, %guild_id, "wiki search failed");
                Vec::new()
            },
            Err(e) => return Err(TicketError::Wiki(e.to_string())),
        };

        let hits = merge(&articles, pages, limit);

        let embed = match hits.first() {
            None => embeds::results(&context.wiki, &hits),
            Some(hit) => match hit.source {
                FaqSource::Local { id } => {
                    match articles.iter().find(|a| a.id == id) {
                        Some(article) => {
                            local_embed(app, &context, query, article).await
                        },
                        None => embeds::results(&context.wiki, &hits),
                    }
                },
                FaqSource::Wiki => {
                    wiki_embed(app, &context, query, &hit.path, &hits).await
                },
            },
        };

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}

fn merge(
    articles: &[FaqArticle],
    pages: Vec<wiki::SearchResult>,
    limit: usize,
) -> Vec<FaqHit> {
    let mut seen = HashSet::new();

    let mut hits = articles
        .iter()
        .map(FaqHit::from)
        .filter(|hit| seen.insert(hit.path.clone()))
        .collect::<Vec<_>>();

    hits.extend(
        pages
            .into_iter()
            .map(FaqHit::from)
            .filter(|hit| seen.insert(hit.path.clone())),
    );

    hits.truncate(limit);
    hits
}

async fn local_embed(
    app: &AppState,
    context: &FaqContext,
    query: &str,
    article: &FaqArticle,
) -> CreateEmbed<'static> {
    match answer(app, context.tuning, query, &article.content).await {
        Ok(answer) => embeds::local_answer(article, &answer),
        Err(e) => {
            error!(error = ?e, query, "faq answer failed for a stored article");
            embeds::stored(article)
        },
    }
}

async fn wiki_embed(
    app: &AppState,
    context: &FaqContext,
    query: &str,
    path: &str,
    hits: &[FaqHit],
) -> CreateEmbed<'static> {
    let Some(page) = fetch(app, &context.wiki, path).await else {
        return embeds::results(&context.wiki, hits);
    };

    match answer(app, context.tuning, query, &page.content).await {
        Ok(answer) => embeds::answer(&context.wiki, &page, &answer),
        Err(e) => {
            error!(error = ?e, query, "faq answer failed for a wiki page");
            embeds::page(&context.wiki, &page)
        },
    }
}

async fn fetch(app: &AppState, config: &WikiConfig, path: &str) -> Option<Page> {
    match wiki::page(&app.http, config, path).await {
        Ok(page) => Some(page),
        Err(e) => {
            warn!(error = ?e, path, "could not read wiki page source");
            None
        },
    }
}

fn limit_as_i64(limit: usize) -> i64 {
    i64::try_from(limit).unwrap_or(5)
}
