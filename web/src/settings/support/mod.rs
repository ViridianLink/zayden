mod faq;
mod settings;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::see_other;
use topcoat::router::request::uri;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::runtime::signal;
use topcoat::view::{View, ViewExt, component, view};

use super::faq::{
    ArticleForm,
    DeleteArticleForm,
    delete_faq_article,
    parse_article_id,
    save_faq_article,
};
use super::{Lists, Submission, ensure_path_guild, settings_page};
use crate::guild::GuildError;
use crate::guild::dto::SupportSection;
use crate::guild::faq::{
    FaqSettingsForm,
    FaqTuningForm,
    FaqWikiKeyForm,
    save_faq_settings,
    save_faq_tuning,
    save_faq_wiki_key,
};
use crate::guild::support::{
    AddHelperLinkForm,
    IdleSettingsForm,
    RemoveHelperLinkForm,
    StaleSettingsForm,
    SuggestionsSettingsForm,
    SupportRoleForm,
    SupportSettingsForm,
    add_helper_link,
    add_support_role,
    remove_helper_link,
    remove_support_role,
    save_idle_settings,
    save_stale_settings,
    save_suggestions_settings,
    save_support_settings,
};
use crate::shell::GuildId;

const SLUG: &str = "support";

const TICKETS: &str = "tickets";
const IDLE: &str = "idle";
const STALE: &str = "stale";
const SUGGESTIONS: &str = "suggestions";
const WIKI: &str = "wiki";
const WIKI_KEY: &str = "wiki-key";
const WIKI_TUNING: &str = "wiki-tuning";
const ADD_ROLE: &str = "add-role";
const REMOVE_ROLE: &str = "remove-role";
const ADD_LINK: &str = "add-link";
const REMOVE_LINK: &str = "remove-link";
const SAVE_ARTICLE: &str = "save-article";
const DELETE_ARTICLE: &str = "delete-article";

const RELOADING: [&str; 4] = [ADD_ROLE, REMOVE_ROLE, ADD_LINK, REMOVE_LINK];
const REMOVE_ROLE_QUERY: &str = "remove-role";

const FLAGS: [(&str, &str); 6] = [
    (ADD_ROLE, "role-added"),
    (REMOVE_ROLE, "role-removed"),
    (ADD_LINK, "link-added"),
    (REMOVE_LINK, "link-removed"),
    (SAVE_ARTICLE, "article-created"),
    (DELETE_ARTICLE, "article-deleted"),
];

fn sent_form(cx: &Cx, pairs: &[(String, String)]) -> &'static str {
    let sent = |name: &str| pairs.iter().any(|(key, _)| key == name);

    [
        ("support_channel_id", TICKETS),
        ("idle_enabled", IDLE),
        ("stale_enabled", STALE),
        ("suggestions_channel_id", SUGGESTIONS),
        ("wiki_url", WIKI),
        ("keep_wiki_api_key", WIKI_KEY),
        ("max_results", WIKI_TUNING),
        ("link", ADD_LINK),
        ("user_id", REMOVE_LINK),
        ("title", SAVE_ARTICLE),
        ("id", DELETE_ARTICLE),
    ]
    .into_iter()
    .find(|(field, _)| sent(field))
    .map(|(_, form)| form)
    .or_else(|| {
        sent("role_id").then(
            || {
                if removes_role(cx) { REMOVE_ROLE } else { ADD_ROLE }
            },
        )
    })
    .unwrap_or(TICKETS)
}

fn query_has(cx: &Cx, name: &str, value: Option<&str>) -> bool {
    uri(cx).query().is_some_and(|query| {
        url::form_urlencoded::parse(query.as_bytes()).any(|(key, sent)| {
            key == name && value.is_none_or(|value| sent == value)
        })
    })
}

fn removes_role(cx: &Cx) -> bool {
    query_has(cx, REMOVE_ROLE_QUERY, None)
}

fn flag_for(form: &str, pairs: &[(String, String)]) -> Option<&'static str> {
    let creates = || {
        pairs
            .iter()
            .find(|(key, _)| key == "id")
            .is_none_or(|(_, id)| parse_article_id(id).is_none())
    };

    FLAGS
        .into_iter()
        .find(|(flagged, _)| *flagged == form)
        .filter(|(flagged, _)| *flagged != SAVE_ARTICLE || creates())
        .map(|(_, flag)| flag)
}

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(support).page(submit)
}

#[page("/guild/{guild_id}/settings/support")]
async fn support(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let submission = FLAGS
        .into_iter()
        .find(|(_, flag)| query_has(cx, flag, Some("1")))
        .map(|(form, _)| Submission::succeeded(form));

    Ok(view! {
        settings_page(guild_id: guild_id, slug: SLUG, submission: submission.as_ref())
    })
}

#[page(POST "/guild/{guild_id}/settings/support")]
async fn submit(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let form = sent_form(cx, &pairs);
    let values = pairs.clone();
    let flag = flag_for(form, &pairs);
    let result = save(cx, guild_id, form, pairs).await;
    if let (Ok(()), Some(flag)) = (&result, flag) {
        return Err(see_other(format!("{}?{flag}=1", form_action(guild_id))).into());
    }
    let submission = if RELOADING.contains(&form) {
        Submission::reloading(form, result)?
    } else {
        Submission::new(form, values, result)?
    };

    Ok(view! {
        (submission.status())
        settings_page(guild_id: guild_id, slug: SLUG, submission: Some(&submission))
    })
}

macro_rules! checked {
    ($form:ty, $pairs:expr, $guild_id:expr) => {{
        let form = <$form>::from_pairs($pairs)?;
        ensure_path_guild(&form.guild, $guild_id)?;
        form
    }};
}

async fn save(
    cx: &Cx,
    guild_id: &str,
    form: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    match form {
        IDLE => {
            let form = checked!(IdleSettingsForm, pairs, guild_id);
            save_idle_settings(cx, &form).await
        },
        STALE => {
            let form = checked!(StaleSettingsForm, pairs, guild_id);
            save_stale_settings(cx, &form).await
        },
        SUGGESTIONS => {
            let form = checked!(SuggestionsSettingsForm, pairs, guild_id);
            save_suggestions_settings(cx, &form).await
        },
        WIKI => {
            let form = checked!(FaqSettingsForm, pairs, guild_id);
            save_faq_settings(cx, &form).await
        },
        WIKI_KEY => {
            let form = checked!(FaqWikiKeyForm, pairs, guild_id);
            save_faq_wiki_key(cx, &form).await
        },
        WIKI_TUNING => {
            let form = checked!(FaqTuningForm, pairs, guild_id);
            save_faq_tuning(cx, &form).await
        },
        ADD_ROLE => {
            let form = checked!(SupportRoleForm, pairs, guild_id);
            add_support_role(cx, &form).await
        },
        REMOVE_ROLE => {
            let form = checked!(SupportRoleForm, pairs, guild_id);
            remove_support_role(cx, &form).await
        },
        ADD_LINK => {
            let form = checked!(AddHelperLinkForm, pairs, guild_id);
            add_helper_link(cx, &form).await
        },
        REMOVE_LINK => {
            let form = checked!(RemoveHelperLinkForm, pairs, guild_id);
            remove_helper_link(cx, &form).await
        },
        SAVE_ARTICLE => {
            let form = checked!(ArticleForm, pairs, guild_id);
            Ok(save_faq_article(cx, &form).await?)
        },
        DELETE_ARTICLE => {
            let form = checked!(DeleteArticleForm, pairs, guild_id);
            Ok(delete_faq_article(cx, &form).await?)
        },
        _ => {
            let form = checked!(SupportSettingsForm, pairs, guild_id);
            save_support_settings(cx, &form).await
        },
    }
}

#[component]
pub(super) async fn tab(
    cx: &Cx,
    guild_id: &str,
    settings: &SupportSection,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let article_sent = Submission::of(submission, SAVE_ARTICLE).is_some()
        || Submission::of(submission, DELETE_ARTICLE).is_some();
    let faq_open = signal(cx, move || article_sent);

    Ok(view! {
        <div class="segmented" role="tablist">
            <button
                type="button"
                :class=$(if faq_open.get() { "seg" } else { "seg active" })
                @click=$(|_e| faq_open.set(false))
            >
                "Settings"
            </button>
            <button
                type="button"
                :class=$(if faq_open.get() { "seg active" } else { "seg" })
                @click=$(|_e| faq_open.set(true))
            >
                "FAQ"
            </button>
        </div>
        settings::pane(
            guild_id: guild_id,
            settings: settings,
            lists: lists,
            submission: submission,
            faq_open: &faq_open
        )
        faq::pane(guild_id: guild_id, submission: submission, faq_open: &faq_open)
    }
    .boxed())
}

fn form_action(guild_id: &str) -> String {
    super::action(guild_id, SLUG)
}
