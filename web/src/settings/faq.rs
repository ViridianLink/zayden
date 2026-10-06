mod error;

use error::ArticleError;
use ticket::{FaqArticle, NewArticle};
use topcoat::context::Cx;

use crate::form::fold;
use crate::guild::{GuildError, admin_app};

const LIST_LIMIT: i64 = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FaqArticleInfo {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) summary: String,
    pub(super) content: String,
    pub(super) category: String,
    pub(super) tags: String,
    pub(super) generated: bool,
    pub(super) source_thread_id: Option<String>,
    pub(super) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ArticleForm {
    pub(super) guild: String,
    id: String,
    title: String,
    summary: String,
    content: String,
    category: String,
    tags: String,
}

impl ArticleForm {
    pub(super) fn from_pairs(
        pairs: Vec<(String, String)>,
    ) -> Result<Self, GuildError> {
        let [guild, id, title, summary, content, category, tags] = fold(pairs, [
            "guild", "id", "title", "summary", "content", "category", "tags",
        ])?;
        Ok(Self { guild, id, title, summary, content, category, tags })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeleteArticleForm {
    pub(super) guild: String,
    id: String,
}

impl DeleteArticleForm {
    pub(super) fn from_pairs(
        pairs: Vec<(String, String)>,
    ) -> Result<Self, GuildError> {
        let [guild, id] = fold(pairs, ["guild", "id"])?;
        Ok(Self { guild, id })
    }
}

pub(super) async fn list_faq_articles(
    cx: &Cx,
    guild: &str,
) -> Result<Vec<FaqArticleInfo>, ArticleError> {
    let (guild_id, app) = admin_app(cx, guild).await?;

    let articles = FaqArticle::list(&app.db, guild_id, LIST_LIMIT).await?;

    Ok(articles.iter().map(info).collect())
}

pub(super) async fn save_faq_article(
    cx: &Cx,
    form: &ArticleForm,
) -> Result<(), ArticleError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let title = form.title.trim();
    let content = form.content.trim();

    if title.is_empty() || content.is_empty() {
        return Err(ArticleError::TitleAndBodyRequired);
    }

    let category = form.category.trim();
    let tags = parse_tags(&form.tags);
    let article = NewArticle {
        title,
        summary: form.summary.trim(),
        content,
        category: (!category.is_empty()).then_some(category),
        tags: &tags,
    };

    match parse_article_id(&form.id) {
        None => FaqArticle::create(&app.db, guild_id, article).await.map(|_| ())?,
        Some(id) => FaqArticle::update(&app.db, guild_id, id, article)
            .await?
            .map(|_| ())
            .ok_or(ArticleError::NoSuchArticle)?,
    }

    Ok(())
}

pub(super) async fn delete_faq_article(
    cx: &Cx,
    form: &DeleteArticleForm,
) -> Result<(), ArticleError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let id = parse_article_id(&form.id).ok_or(ArticleError::NoSuchArticle)?;

    FaqArticle::delete(&app.db, guild_id, id).await?;

    Ok(())
}

pub(super) fn parse_article_id(id: &str) -> Option<i32> {
    id.trim().parse().ok()
}

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

fn info(article: &FaqArticle) -> FaqArticleInfo {
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
