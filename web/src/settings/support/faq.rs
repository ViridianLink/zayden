use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, StatusCode, page, path_param};
use topcoat::view::{View, ViewExt, component, suspense, view};

use super::Pane;
use crate::components::confirm::confirm_button;
use crate::components::data_table::{data_cell, data_row, data_table};
use crate::components::empty_state::empty_state;
use crate::components::save_bar::save_bar;
use crate::components::shape_skeleton::{SkeletonShape, shape_skeleton};
use crate::guild::{GuildError, GuildForm};
use crate::settings::faq::{
    ArticleError,
    ArticleForm,
    FaqArticleInfo,
    delete_faq_article,
    get_faq_article,
    list_faq_articles,
    parse_article_id,
    save_faq_article,
};
use crate::settings::fields::{form_summary, text_row, textarea_row};
use crate::settings::state::{Done, Failure, PageState, succeed};
use crate::settings::{Page, ensure_path_guild, frame, load_error};
use crate::shell::GuildId;

path_param!(article_id);

const PAGE: Page = Page::Support(Pane::Faq);
const ARTICLE_FORM: &str = "faq-article";
const DELETE_FORM: &str = "faq-delete";
const ARTICLES_SECTION: &str = "faq-articles";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(article_index)
        .page(new_article_page)
        .page(create_article)
        .page(edit_article)
        .page(update_article)
        .page(delete_article)
}

fn article_href(guild_id: &str, id: impl std::fmt::Display) -> String {
    format!("{}/{id}", PAGE.href(guild_id))
}

fn article_failure(
    form: &'static str,
    values: Vec<(String, String)>,
    error: ArticleError,
    missing: Option<&str>,
) -> Result<Failure> {
    Ok(match error {
        ArticleError::Guild(error) => {
            Failure::new(form, values, &error.redirect_unauthenticated()?)
        },
        ArticleError::TitleAndBodyRequired => {
            Failure::with_message(form, values, &error.to_string(), missing)
        },
        ArticleError::NoSuchArticle => {
            Failure::with_message(form, values, &error.to_string(), None)
        },
    })
}

#[page("/guild/{guild_id}/support/faq")]
async fn article_index(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! {
        frame(
            guild_id: guild_id,
            page: PAGE,
            state: &state,
            suspense(
                fallback: view! {
                    <div class="skeleton-list">
                        shape_skeleton(shape: SkeletonShape::Row, count: 4)
                    </div>
                },
                article_list(guild_id: guild_id)
            )
        )
    })
}

#[component]
async fn article_list(cx: &Cx, guild_id: &str) -> Result<impl View> {
    let listed = list_faq_articles(cx, guild_id).await.map_err(GuildError::from);
    let new_href = format!("{}/new", PAGE.href(guild_id));

    Ok(view! {
        match listed {
            Err(error) => load_error(guild_id: guild_id, page: PAGE, error: &error),
            Ok(articles) => {
                <section
                    class="settings-section"
                    id=(ARTICLES_SECTION)
                    aria-labelledby="faq-articles-title"
                >
                    <h2 class="label" id="faq-articles-title">"Articles"</h2>
                    <p class="page-lead">
                        "Articles answer \"/ticket faq ask\" and the automatic ticket triage, alongside the wiki. Articles written from solved tickets go live as soon as they are written, so review them here."
                    </p>
                    if articles.is_empty() {
                        empty_state(
                            title: "No FAQ articles yet",
                            text: "Write one now, or turn on article writing on the Wiki page to collect them from solved tickets.",
                            action: Some("New article"),
                            href: Some(&new_href)
                        )
                    } else {
                        <div class="settings-actions">
                            <a class="btn btn-primary" href=(new_href.as_str())>
                                "New article"
                            </a>
                        </div>
                        data_table(
                            caption: "FAQ articles",
                            columns: &["Title", "Category", "Updated", "Source"],
                            #[key(article.id.as_str())]
                            for article in &articles {
                                data_row(
                                    data_cell(
                                        label: "Title",
                                        header: true,
                                        <a href=(article_href(guild_id, &article.id))>
                                            (article.title.as_str())
                                        </a>
                                    )
                                    data_cell(label: "Category", (article.category.as_str()))
                                    data_cell(label: "Updated", (article.updated_at.as_str()))
                                    data_cell(label: "Source", source(article: article))
                                )
                            }
                        )
                    }
                </section>
            }
        }
    }
    .boxed())
}

#[component]
async fn source(article: &FaqArticleInfo) -> Result<impl View> {
    Ok(view! {
        match (article.generated, article.source_thread_id.as_deref()) {
            (true, Some(thread)) => (format!("Solved ticket, thread {thread}")),
            (true, None) => "Solved ticket",
            (false, _) => "Written here",
        }
    })
}

#[page("/guild/{guild_id}/support/faq/new")]
async fn new_article_page(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! {
        frame(
            guild_id: guild_id,
            page: PAGE,
            state: &state,
            article_form(guild_id: guild_id, article: None, state: &state)
        )
    })
}

#[page(POST "/guild/{guild_id}/support/faq/new")]
async fn create_article(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let (result, missing) = save(cx, guild_id, pairs, None).await;
    let failure = match result {
        Ok(id) => {
            return Err(succeed(cx, &Done {
                page: article_href(guild_id, id),
                section: Some(ARTICLE_FORM),
                message: "Article created.",
            }));
        },
        Err(error) => article_failure(ARTICLE_FORM, values, error, missing)?,
    };
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        frame(
            guild_id: guild_id,
            page: PAGE,
            state: &state,
            article_form(guild_id: guild_id, article: None, state: &state)
        )
    })
}

async fn save(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
    id: Option<i32>,
) -> (std::result::Result<i32, ArticleError>, Option<&'static str>) {
    let form = match ArticleForm::from_pairs(pairs) {
        Ok(form) => form,
        Err(error) => return (Err(error.into()), None),
    };
    if let Err(error) = ensure_path_guild(&form.guild, guild_id) {
        return (Err(error.into()), None);
    }
    (save_faq_article(cx, &form, id).await, form.missing())
}

fn requested_id(cx: &Cx) -> Option<i32> {
    parse_article_id(path_param::<ArticleId>(cx))
}

#[page("/guild/{guild_id}/support/faq/{article_id}")]
async fn edit_article(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let id = requested_id(cx);
    let state = PageState::load(cx);

    Ok(view! {
        frame(
            guild_id: guild_id,
            page: PAGE,
            state: &state,
            article_editor(guild_id: guild_id, id: id, state: &state)
        )
    })
}

#[page(POST "/guild/{guild_id}/support/faq/{article_id}")]
async fn update_article(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let id = requested_id(cx);
    let values = pairs.clone();
    let (result, missing) = match id {
        Some(id) => save(cx, guild_id, pairs, Some(id)).await,
        None => (Err(ArticleError::NoSuchArticle), None),
    };
    let failure = match result {
        Ok(id) => {
            return Err(succeed(cx, &Done {
                page: article_href(guild_id, id),
                section: Some(ARTICLE_FORM),
                message: "Article saved.",
            }));
        },
        Err(error) => article_failure(ARTICLE_FORM, values, error, missing)?,
    };
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        frame(
            guild_id: guild_id,
            page: PAGE,
            state: &state,
            article_editor(guild_id: guild_id, id: id, state: &state)
        )
    })
}

#[page(POST "/guild/{guild_id}/support/faq/{article_id}/delete")]
async fn delete_article(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let id = requested_id(cx);
    let values = pairs.clone();
    let result = async {
        let form = GuildForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        let id = id.ok_or(ArticleError::NoSuchArticle)?;
        delete_faq_article(cx, guild_id, id).await
    }
    .await;
    let failure = match result {
        Ok(()) => {
            return Err(succeed(cx, &Done {
                page: PAGE.href(guild_id),
                section: None,
                message: "Article deleted.",
            }));
        },
        Err(error) => article_failure(DELETE_FORM, values, error, None)?,
    };
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        frame(
            guild_id: guild_id,
            page: PAGE,
            state: &state,
            article_editor(guild_id: guild_id, id: id, state: &state)
        )
    })
}

#[component]
async fn article_editor(
    cx: &Cx,
    guild_id: &str,
    id: Option<i32>,
    state: &PageState,
) -> Result<impl View> {
    let loaded = match id {
        Some(id) => {
            get_faq_article(cx, guild_id, id).await.map_err(GuildError::from)
        },
        None => Ok(None),
    };
    let back = PAGE.href(guild_id);

    Ok(view! {
        match loaded {
            Err(error) => load_error(
                guild_id: guild_id,
                page: PAGE,
                error: &error,
                failure: state.any_failure()
            ),
            Ok(None) => {
                (StatusCode::NOT_FOUND)
                if let Some(failure) = state.any_failure() {
                    form_summary(form: "page", message: failure)
                }
                <section class="error-panel" aria-labelledby="faq-missing-title">
                    <h2 class="error-title" id="faq-missing-title">
                        "Article not found"
                    </h2>
                    <p class="error-text">
                        "This FAQ article doesn't exist. It may have been deleted."
                    </p>
                    <div class="error-actions">
                        <a href=(back.as_str()) class="btn btn-primary">
                            "Back to FAQ articles"
                        </a>
                    </div>
                </section>
            }
            Ok(Some(article)) => {
                article_form(guild_id: guild_id, article: Some(&article), state: state)
                delete_section(guild_id: guild_id, article: &article, state: state)
            }
        }
    }
    .boxed())
}

#[component]
async fn article_form(
    guild_id: &str,
    article: Option<&FaqArticleInfo>,
    state: &PageState,
) -> Result<impl View> {
    let sent = state.sent(ARTICLE_FORM);
    let title = sent.value("title", article.map(|a| a.title.as_str()));
    let summary = sent.value("summary", article.map(|a| a.summary.as_str()));
    let category = sent.value("category", article.map(|a| a.category.as_str()));
    let tags = sent.value("tags", article.map(|a| a.tags.as_str()));
    let content = sent.value("content", article.map(|a| a.content.as_str()));
    let action = article.map_or_else(
        || format!("{}/new", PAGE.href(guild_id)),
        |article| article_href(guild_id, &article.id),
    );
    let (legend, save_label) = match article {
        Some(_) => ("Article", "Save article"),
        None => ("New article", "Create article"),
    };

    Ok(view! {
        <form
            id=(ARTICLE_FORM)
            method="post"
            action=(action.as_str())
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = sent.summary() {
                form_summary(form: ARTICLE_FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>(legend)</legend>
                if let Some(article) = article {
                    <p class="field-hint">
                        "Updated "
                        (article.updated_at.as_str())
                        if let Some(thread) = article.source_thread_id.as_deref() {
                            (format!(" \u{2022} written from thread {thread}"))
                        }
                    </p>
                }
                text_row(
                    form: ARTICLE_FORM,
                    name: "title",
                    label: "Title",
                    value: title,
                    error: sent.error("title"),
                    placeholder: Some("Fixing Radarr error 502"),
                    required: true
                )
                text_row(
                    form: ARTICLE_FORM,
                    name: "summary",
                    label: "Summary",
                    value: summary,
                    help: Some("One sentence, shown in search results."),
                    error: sent.error("summary")
                )
                text_row(
                    form: ARTICLE_FORM,
                    name: "category",
                    label: "Category",
                    value: category,
                    error: sent.error("category")
                )
                text_row(
                    form: ARTICLE_FORM,
                    name: "tags",
                    label: "Tags",
                    value: tags,
                    help: Some("Comma separated."),
                    error: sent.error("tags"),
                    placeholder: Some("comma, separated")
                )
                textarea_row(
                    form: ARTICLE_FORM,
                    name: "content",
                    label: "Body (Markdown)",
                    value: content,
                    rows: 14,
                    error: sent.error("content"),
                    required: true
                )
            </fieldset>
            save_bar(label: save_label, notice: state.notice_for(ARTICLE_FORM))
        </form>
    }
    .boxed())
}

#[component]
async fn delete_section(
    guild_id: &str,
    article: &FaqArticleInfo,
    state: &PageState,
) -> Result<impl View> {
    let action = format!("{}/delete", article_href(guild_id, &article.id));
    let confirm_object = format!("\u{201c}{}\u{201d}", article.title);
    let failed = state.sent(DELETE_FORM).summary();
    let confirm_id = format!("faq-{}-delete", article.id);

    Ok(view! {
        <form id=(DELETE_FORM) method="post" action=(action.as_str()) data-pending="">
            if let Some(message) = failed {
                form_summary(
                    form: DELETE_FORM,
                    message: message,
                    outcome: "Not deleted"
                )
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Delete"</legend>
                <p class="page-lead">
                    "Deleting removes the article for everyone, including the wiki copy."
                </p>
                confirm_button(
                    id: &confirm_id,
                    label: "Delete article",
                    prompt: "This removes the article for everyone, including the wiki copy. It cannot be undone.",
                    confirm: "Delete article",
                    object: Some(&confirm_object)
                )
            </fieldset>
        </form>
    }
    .boxed())
}
