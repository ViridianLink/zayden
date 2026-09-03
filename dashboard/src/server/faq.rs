use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::server_err,
    crate::server::guild::admin_app,
    ticket::{FaqArticle, NewArticle},
};

use crate::dto::FaqArticleInfo;

#[cfg(feature = "ssr")]
const LIST_LIMIT: i64 = 200;

#[server]
pub async fn list_faq_articles(
    guild: String,
) -> Result<Vec<FaqArticleInfo>, ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let articles =
        FaqArticle::list(&app.db, guild_id, LIST_LIMIT).await.map_err(server_err)?;

    Ok(articles.iter().map(view).collect())
}

#[server]
pub async fn save_faq_article(
    guild: String,
    id: String,
    title: String,
    summary: String,
    content: String,
    category: String,
    tags: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let title = title.trim();
    let content = content.trim();

    if title.is_empty() || content.is_empty() {
        return Err(ServerFnError::new("A title and a body are both required."));
    }

    let summary = summary.trim();
    let category = category.trim();
    let tags = parse_tags(&tags);

    let article = NewArticle {
        title,
        summary,
        content,
        category: (!category.is_empty()).then_some(category),
        tags: &tags,
    };

    match parse_article_id(&id) {
        None => FaqArticle::create(&app.db, guild_id, article)
            .await
            .map(|_| ())
            .map_err(server_err),
        Some(id) => match FaqArticle::update(&app.db, guild_id, id, article)
            .await
            .map_err(server_err)?
        {
            Some(_) => Ok(()),
            None => Err(ServerFnError::new("That article no longer exists.")),
        },
    }
}

#[server]
pub async fn delete_faq_article(
    guild: String,
    id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let id = parse_article_id(&id)
        .ok_or_else(|| ServerFnError::new("That article no longer exists."))?;

    FaqArticle::delete(&app.db, guild_id, id).await.map(|_| ()).map_err(server_err)
}

#[cfg(feature = "ssr")]
fn parse_article_id(id: &str) -> Option<i32> {
    id.trim().parse().ok()
}

#[cfg(feature = "ssr")]
fn parse_tags(tags: &str) -> Vec<String> {
    let mut parsed: Vec<String> = Vec::new();

    for tag in tags.split(',') {
        let tag = tag.trim().to_lowercase();

        if !tag.is_empty() && !parsed.contains(&tag) {
            parsed.push(tag);
        }
    }

    parsed
}

#[cfg(feature = "ssr")]
fn view(article: &FaqArticle) -> FaqArticleInfo {
    FaqArticleInfo {
        id: article.id.to_string(),
        title: article.title.clone(),
        summary: article.summary.clone(),
        content: article.content.clone(),
        category: article.category.clone().unwrap_or_default(),
        tags: article.tags.join(", "),
        generated: article.generated,
        source_thread_id: article.source_thread_id.map(|id| id.to_string()),
        updated_at: article.updated_at.to_jiff().to_string(),
    }
}
