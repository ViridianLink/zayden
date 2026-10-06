use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::Signal;
use topcoat::view::{View, ViewExt, component, suspense, view};

use super::{DELETE_ARTICLE, SAVE_ARTICLE, form_action};
use crate::components::confirm::confirm_button;
use crate::components::settings::{save_button, save_feedback, setting_field};
use crate::components::shape_skeleton::{SkeletonShape, shape_skeleton};
use crate::settings::faq::{FaqArticleInfo, list_faq_articles, parse_article_id};
use crate::settings::{Submission, shown};
use crate::util::server_error_text;

#[component]
pub(super) async fn pane(
    guild_id: &str,
    submission: Option<&Submission>,
    faq_open: &Signal<bool>,
) -> Result<impl View> {
    let action = form_action(guild_id);
    let saved = Submission::of(submission, SAVE_ARTICLE);
    let deleted = Submission::of(submission, DELETE_ARTICLE);
    let new_article = saved.filter(|submitted| {
        parse_article_id(shown(Some(*submitted), "id", None)).is_none()
    });

    Ok(view! {
        <fieldset class="settings-section" :hidden=$(!faq_open.get())>
            <p class="page-lead">
                "Articles \"/ticket faq ask\" and the automated triage search, alongside the wiki. Articles written from solved tickets go live as soon as they are generated, so review them here."
            </p>
            if let Some(submitted) = saved {
                save_feedback(outcome: submitted.outcome())
            }
            if let Some(submitted) = deleted {
                save_feedback(outcome: submitted.outcome())
            }
            <details class="setting-field" open=(new_article.is_some())>
                <summary>"New article"</summary>
                article_form(
                    guild_id: guild_id,
                    action: &action,
                    id: "",
                    title: shown(new_article, "title", None),
                    summary: shown(new_article, "summary", None),
                    category: shown(new_article, "category", None),
                    tags: shown(new_article, "tags", None),
                    content: shown(new_article, "content", None)
                )
            </details>
            suspense(
                fallback: view! {
                    <div class="skeleton-list">
                        shape_skeleton(shape: SkeletonShape::Row, count: 4)
                    </div>
                },
                article_list(guild_id: guild_id, action: &action)
            )
        </fieldset>
    }
    .boxed())
}

#[component]
async fn article_list(cx: &Cx, guild_id: &str, action: &str) -> Result<impl View> {
    let articles = list_faq_articles(cx, guild_id).await.map_err(server_error_text);

    Ok(view! {
        match articles {
            Err(error) => <p class="error">
                "Failed to load articles: "
                (error)
            </p>,
            Ok(articles) => {
                if articles.is_empty() {
                    <p class="page-lead">"No FAQ articles yet."</p>
                }
                #[key(article.id.as_str())]
                for article in &articles {
                    article_row(guild_id: guild_id, action: action, article: article)
                }
            }
        }
    })
}

#[component]
async fn article_row(
    guild_id: &str,
    action: &str,
    article: &FaqArticleInfo,
) -> Result<impl View> {
    let source = article
        .source_thread_id
        .as_deref()
        .map(|thread| format!(" \u{2022} from thread {thread}"));

    Ok(view! {
        <details class="setting-field">
            <summary>
                (article.title.as_str())
                if article.generated {
                    <span class="chip-label">" generated"</span>
                }
            </summary>
            <p class="field-hint">
                "Updated "
                (article.updated_at.as_str())
                if let Some(source) = source {
                    (source)
                }
            </p>
            article_form(
                guild_id: guild_id,
                action: action,
                id: &article.id,
                title: &article.title,
                summary: &article.summary,
                category: &article.category,
                tags: &article.tags,
                content: &article.content
            )
            <form method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                <input type="hidden" name="id" value=(article.id.as_str())>
                <div class="form-actions">
                    confirm_button(
                        label: "Delete",
                        prompt: "This removes the article for everyone, including the wiki copy. It cannot be undone.",
                        confirm: "Delete article"
                    )
                </div>
            </form>
        </details>
    })
}

#[component]
async fn article_form(
    guild_id: &str,
    action: &str,
    id: &str,
    title: &str,
    summary: &str,
    category: &str,
    tags: &str,
    content: &str,
) -> Result<impl View> {
    Ok(view! {
        <form method="post" action=(action) data-pending="">
            <input type="hidden" name="guild" value=(guild_id)>
            <input type="hidden" name="id" value=(id)>
            setting_field(
                label: "Title",
                name: "title",
                value: title,
                pattern: ".*",
                placeholder: "Fixing Radarr error 502"
            )
            setting_field(
                label: "Summary",
                name: "summary",
                value: summary,
                pattern: ".*",
                placeholder: "One sentence, shown in search results"
            )
            setting_field(
                label: "Category",
                name: "category",
                value: category,
                pattern: ".*"
            )
            setting_field(
                label: "Tags",
                name: "tags",
                value: tags,
                pattern: ".*",
                placeholder: "comma, separated",
                hint: Some("Comma separated.")
            )
            <div class="setting-field">
                <label>"Body (Markdown)"</label>
                <textarea class="input" name="content" rows="14">(content)</textarea>
            </div>
            save_button()
        </form>
    })
}
