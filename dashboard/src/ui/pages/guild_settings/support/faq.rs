use leptos::prelude::*;

use crate::dto::FaqArticleInfo;
use crate::server::faq::{DeleteFaqArticle, SaveFaqArticle, list_faq_articles};
use crate::ui::components::icons::Icon;
use crate::ui::components::settings::{SaveButton, SettingField, save_feedback};

#[component]
pub(crate) fn FaqArticlesPane(guild_id: String) -> impl IntoView {
    let save = ServerAction::<SaveFaqArticle>::new();
    let delete = ServerAction::<DeleteFaqArticle>::new();
    let save_result = save.value();
    let delete_result = delete.value();

    let list_guild_id = guild_id.clone();

    // Re-runs whenever an edit or a delete lands, so the list never shows a
    // stale article.
    let articles = Resource::new(
        move || {
            (list_guild_id.clone(), save.version().get(), delete.version().get())
        },
        |(guild_id, ..)| async move { list_faq_articles(guild_id).await },
    );

    let new_guild_id = guild_id.clone();

    view! {
        <fieldset class="settings-section">
            <p class="page-lead">
                "Articles \"/ticket faq ask\" and the automated triage search, "
                "alongside the wiki. Articles written from solved tickets go "
                "live as soon as they are generated, so review them here."
            </p>
            {move || save_result.get().map(save_feedback)}
            {move || delete_result.get().map(save_feedback)}
            <details class="setting-field">
                <summary>"New article"</summary>
                <ArticleForm
                    guild_id=new_guild_id
                    article=None
                    save=save
                />
            </details>
            <Suspense fallback=|| view! {
                <p class="loading">"Loading articles\u{2026}"</p>
            }>
                {move || articles.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="error">"Failed to load articles: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(articles) if articles.is_empty() => view! {
                        <p class="page-lead">"No FAQ articles yet."</p>
                    }.into_any(),
                    Ok(articles) => {
                        let gid = guild_id.clone();

                        articles
                            .into_iter()
                            .map(|article| view! {
                                <ArticleRow
                                    guild_id=gid.clone()
                                    article=article
                                    save=save
                                    delete=delete
                                />
                            })
                            .collect_view()
                            .into_any()
                    },
                })}
            </Suspense>
        </fieldset>
    }
}

#[component]
fn ArticleRow(
    guild_id: String,
    article: FaqArticleInfo,
    save: ServerAction<SaveFaqArticle>,
    delete: ServerAction<DeleteFaqArticle>,
) -> impl IntoView {
    let id = article.id.clone();
    let title = article.title.clone();
    let updated_at = article.updated_at.clone();
    let generated = article.generated;
    let source = article.source_thread_id.clone();
    let delete_guild_id = guild_id.clone();

    view! {
        <details class="setting-field">
            <summary>
                {title}
                {generated.then(|| view! { <span class="chip-label">" generated"</span> })}
            </summary>
            <p class="field-hint">
                "Updated " {updated_at}
                {source.map(|thread| format!(" \u{2022} from thread {thread}"))}
            </p>
            <ArticleForm
                guild_id=guild_id
                article=Some(article)
                save=save
            />
            <ActionForm action=delete>
                <input type="hidden" name="guild" value=delete_guild_id/>
                <input type="hidden" name="id" value=id/>
                <div class="form-actions">
                    <button type="submit" class="btn btn-danger">
                        <Icon name="x"/>
                        "Delete"
                    </button>
                </div>
            </ActionForm>
        </details>
    }
}

#[component]
fn ArticleForm(
    guild_id: String,
    article: Option<FaqArticleInfo>,
    save: ServerAction<SaveFaqArticle>,
) -> impl IntoView {
    let article = article.unwrap_or_else(empty);

    view! {
        <ActionForm action=save>
            <input type="hidden" name="guild" value=guild_id/>
            <input type="hidden" name="id" value=article.id/>
            <SettingField
                label="Title"
                name="title"
                value=article.title
                pattern=".*"
                placeholder="Fixing Radarr error 502"
            />
            <SettingField
                label="Summary"
                name="summary"
                value=article.summary
                pattern=".*"
                placeholder="One sentence, shown in search results"
            />
            <SettingField
                label="Category"
                name="category"
                value=article.category
                pattern=".*"
            />
            <SettingField
                label="Tags"
                name="tags"
                value=article.tags
                pattern=".*"
                placeholder="comma, separated"
                hint="Comma separated."
            />
            <div class="setting-field">
                <label>"Body (Markdown)"</label>
                <textarea name="content" rows="14">{article.content}</textarea>
            </div>
            <SaveButton/>
        </ActionForm>
    }
}

const fn empty() -> FaqArticleInfo {
    FaqArticleInfo {
        id: String::new(),
        title: String::new(),
        summary: String::new(),
        content: String::new(),
        category: String::new(),
        tags: String::new(),
        generated: false,
        source_thread_id: None,
        updated_at: String::new(),
    }
}
