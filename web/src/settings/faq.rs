mod error;

pub(super) use error::ArticleError;
use ticket::{FaqArticle, NewArticle};
use topcoat::context::Cx;

use crate::form::fold;
use crate::guild::{GuildError, admin_app};

const LIST_LIMIT: i64 = 200;
const TIME_FORMAT: &str = "%-d %b %Y, %H:%M UTC";

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
        let [guild, title, summary, content, category, tags] = fold(pairs, [
            "guild", "title", "summary", "content", "category", "tags",
        ])?;
        Ok(Self { guild, title, summary, content, category, tags })
    }

    pub(super) fn missing(&self) -> Option<&'static str> {
        if self.title.trim().is_empty() {
            Some("title")
        } else if self.content.trim().is_empty() {
            Some("content")
        } else {
            None
        }
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

pub(super) async fn get_faq_article(
    cx: &Cx,
    guild: &str,
    id: i32,
) -> Result<Option<FaqArticleInfo>, ArticleError> {
    let (guild_id, app) = admin_app(cx, guild).await?;

    Ok(FaqArticle::get(&app.db, guild_id, id).await?.as_ref().map(info))
}

pub(super) async fn save_faq_article(
    cx: &Cx,
    form: &ArticleForm,
    id: Option<i32>,
) -> Result<i32, ArticleError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    if form.missing().is_some() {
        return Err(ArticleError::TitleAndBodyRequired);
    }

    let category = form.category.trim();
    let tags = parse_tags(&form.tags);
    let article = NewArticle {
        title: form.title.trim(),
        summary: form.summary.trim(),
        content: form.content.trim(),
        category: (!category.is_empty()).then_some(category),
        tags: &tags,
    };

    let saved = match id {
        None => FaqArticle::create(&app.db, guild_id, article).await?,
        Some(id) => FaqArticle::update(&app.db, guild_id, id, article)
            .await?
            .ok_or(ArticleError::NoSuchArticle)?,
    };

    Ok(saved.id)
}

pub(super) async fn delete_faq_article(
    cx: &Cx,
    guild: &str,
    id: i32,
) -> Result<(), ArticleError> {
    let (guild_id, app) = admin_app(cx, guild).await?;

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
        updated_at: article.updated_at.to_jiff().strftime(TIME_FORMAT).to_string(),
    }
}
