use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::Pane;
use crate::components::save_bar::save_bar;
use crate::guild::dto::{FaqSection, SupportSection};
use crate::guild::faq::{
    FaqWikiKeyForm,
    WikiSettingsForm,
    save_faq_wiki_key,
    save_wiki_settings,
};
use crate::guild::parse::{parse_flag, parse_optional};
use crate::guild::support::{
    SuggestionsSettingsForm,
    TicketSettingsForm,
    save_suggestions_settings,
    save_ticket_settings,
};
use crate::settings::fields::{
    Range,
    channels,
    form_summary,
    forum_tags,
    select_row,
    text_row,
    toggle_row,
};
use crate::settings::state::{Done, PageState, settle};
use crate::settings::{Lists, Page, TEXT_KINDS, ensure_path_guild, settings_page};
use crate::shell::GuildId;

const TICKETS: Page = Page::Support(Pane::Tickets);
const SUGGESTIONS: Page = Page::Support(Pane::Suggestions);
const WIKI: Page = Page::Support(Pane::Wiki);

const TICKET_FORM: &str = "ticket-settings";
const SUGGESTION_FORM: &str = "suggestion-settings";
const WIKI_FORM: &str = "wiki-settings";
const KEY_FORM: &str = "wiki-key";

const IDLE_RANGE: Range = Range { min: 3_600, max: Some(2_592_000) };

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(tickets_page)
        .page(save_tickets)
        .page(suggestions_page)
        .page(save_suggestions)
        .page(wiki_page)
        .page(save_wiki)
        .page(save_key)
}

#[page("/guild/{guild_id}/support")]
async fn tickets_page(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: TICKETS, state: &state) })
}

#[page(POST "/guild/{guild_id}/support")]
async fn save_tickets(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = TicketSettingsForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        save_ticket_settings(cx, &form).await
    }
    .await;
    let failure = settle(cx, TICKET_FORM, values, result, &Done {
        page: TICKETS.href(guild_id),
        section: Some(TICKET_FORM),
        message: "Ticket settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: TICKETS, state: &state)
    })
}

#[page("/guild/{guild_id}/support/suggestions")]
async fn suggestions_page(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: SUGGESTIONS, state: &state) })
}

#[page(POST "/guild/{guild_id}/support/suggestions")]
async fn save_suggestions(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = SuggestionsSettingsForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        save_suggestions_settings(cx, &form).await
    }
    .await;
    let failure = settle(cx, SUGGESTION_FORM, values, result, &Done {
        page: SUGGESTIONS.href(guild_id),
        section: Some(SUGGESTION_FORM),
        message: "Suggestion settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: SUGGESTIONS, state: &state)
    })
}

#[page("/guild/{guild_id}/support/wiki")]
async fn wiki_page(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: WIKI, state: &state) })
}

#[page(POST "/guild/{guild_id}/support/wiki")]
async fn save_wiki(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = WikiSettingsForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        save_wiki_settings(cx, &form).await
    }
    .await;
    let failure = settle(cx, WIKI_FORM, values, result, &Done {
        page: WIKI.href(guild_id),
        section: Some(WIKI_FORM),
        message: "Wiki settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: WIKI, state: &state)
    })
}

fn key_change(pairs: &[(String, String)]) -> &'static str {
    let sent = |name: &str| {
        pairs.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    };
    let typed = sent("wiki_api_key").and_then(parse_optional).is_some();
    let keep = sent("keep_wiki_api_key").is_none_or(parse_flag);

    match (typed, keep) {
        (true, _) => "Wiki API key saved.",
        (false, true) => "Wiki API key kept.",
        (false, false) => "Wiki API key removed.",
    }
}

#[page(POST "/guild/{guild_id}/support/wiki/wiki-key")]
async fn save_key(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let message = key_change(&pairs);
    let values: Vec<(String, String)> =
        pairs.iter().filter(|(name, _)| name != "wiki_api_key").cloned().collect();
    let result = async {
        let form = FaqWikiKeyForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        save_faq_wiki_key(cx, &form).await
    }
    .await;
    let failure = settle(cx, KEY_FORM, values, result, &Done {
        page: WIKI.href(guild_id),
        section: Some(KEY_FORM),
        message,
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: WIKI, state: &state)
    })
}

#[component]
pub(super) async fn tickets(
    guild_id: &str,
    settings: &SupportSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let s = settings;
    let sent = state.sent(TICKET_FORM);
    let tags = || forum_tags(lists);

    Ok(view! {
        <form
            id=(TICKET_FORM)
            method="post"
            action=(TICKETS.href(guild_id))
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = sent.summary() {
                form_summary(form: TICKET_FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Tickets"</legend>
                <p class="page-lead">
                    "Where tickets open and how a solved ticket is marked."
                </p>
                select_row(
                    form: TICKET_FORM,
                    name: "support_channel_id",
                    label: "Support channel",
                    selected: sent.value(
                        "support_channel_id",
                        s.support_channel_id.as_deref(),
                    ),
                    options: channels(lists, TEXT_KINDS),
                    error: sent.error("support_channel_id")
                )
                select_row(
                    form: TICKET_FORM,
                    name: "solved_tag_id",
                    label: "Solved tag",
                    selected: sent.value("solved_tag_id", s.solved_tag_id.as_deref()),
                    options: tags(),
                    help: Some(
                        "\"/ticket solved\" applies this tag when the support channel is a forum, and otherwise renames the thread.",
                    ),
                    error: sent.error("solved_tag_id")
                )
                select_row(
                    form: TICKET_FORM,
                    name: "closed_tag_id",
                    label: "Closed tag",
                    selected: sent.value("closed_tag_id", s.closed_tag_id.as_deref()),
                    options: tags(),
                    error: sent.error("closed_tag_id")
                )
                text_row(
                    form: TICKET_FORM,
                    name: "solved_archive_secs",
                    label: "Archive solved posts after (seconds)",
                    value: sent.value(
                        "solved_archive_secs",
                        Some(&s.solved_archive_secs),
                    ),
                    help: Some("0 closes the post immediately; -1 leaves it open."),
                    error: sent.error("solved_archive_secs"),
                    range: Some(Range { min: -1, max: None })
                )
            </fieldset>
            <fieldset class="settings-section">
                <legend>"Idle reminders"</legend>
                <p class="page-lead">
                    "Idle reminders watch whose turn it is. If a helper spoke last and the poster has gone quiet for the interval below, the poster is nudged with \"Solved\" and \"Still need help\" buttons. If the poster spoke last, the helper who replied is nudged - or the support roles, if nobody has answered yet. Each side is reminded once per turn, and never again until somebody posts."
                </p>
                toggle_row(
                    form: TICKET_FORM,
                    name: "idle_enabled",
                    label: "Idle reminders",
                    value: sent.flag("idle_enabled", s.idle_enabled),
                    error: sent.error("idle_enabled")
                )
                text_row(
                    form: TICKET_FORM,
                    name: "idle_after_secs",
                    label: "Remind after (seconds of silence)",
                    value: sent.value("idle_after_secs", Some(&s.idle_after_secs)),
                    help: Some(
                        "At least 3600 (one hour), at most 2592000 (30 days). Default 172800 (48 hours).",
                    ),
                    error: sent.error("idle_after_secs"),
                    range: Some(IDLE_RANGE)
                )
                toggle_row(
                    form: TICKET_FORM,
                    name: "idle_close_enabled",
                    label: "Auto-close abandoned posts",
                    value: sent.flag("idle_close_enabled", s.idle_close_enabled),
                    help: Some(
                        "Needs idle reminders on: it acts on a reminder nobody answered. Only posts waiting on the person who opened them are closed - a post waiting on the support team is the team's backlog, not an abandoned ticket. Closing applies the closed tag, posts a note to the poster and archives the post. Any reply at all cancels it.",
                    ),
                    error: sent.error("idle_close_enabled")
                )
                text_row(
                    form: TICKET_FORM,
                    name: "idle_close_after_secs",
                    label: "Close after (seconds without a reply to the reminder)",
                    value: sent.value(
                        "idle_close_after_secs",
                        Some(&s.idle_close_after_secs),
                    ),
                    help: Some(
                        "At least 3600 (one hour), at most 2592000 (30 days). Default 86400 (24 hours).",
                    ),
                    error: sent.error("idle_close_after_secs"),
                    range: Some(IDLE_RANGE)
                )
                <p class="field-hint">
                    "A support role only gets notified if it is mentionable. Role mentions in private ticket threads mostly do not notify at all, since Discord does not pull role members into a private thread. A reminder also un-archives a post Discord had already archived, which is usually the point."
                </p>
            </fieldset>
            <fieldset class="settings-section">
                <legend>"Stale marking"</legend>
                <p class="page-lead">
                    "A quieter signal than closing: a post the poster has left unanswered for the interval below is tagged so the team can see at a glance what has gone cold. It follows the same staff-side exemption as auto-close, and the tag comes off again by itself as soon as anybody posts."
                </p>
                toggle_row(
                    form: TICKET_FORM,
                    name: "stale_enabled",
                    label: "Mark quiet posts stale",
                    value: sent.flag("stale_enabled", s.stale_enabled),
                    error: sent.error("stale_enabled")
                )
                select_row(
                    form: TICKET_FORM,
                    name: "stale_tag_id",
                    label: "Stale tag",
                    selected: sent.value("stale_tag_id", s.stale_tag_id.as_deref()),
                    options: tags(),
                    help: Some("A forum tag is needed to mark a post stale."),
                    error: sent.error("stale_tag_id")
                )
                text_row(
                    form: TICKET_FORM,
                    name: "stale_after_secs",
                    label: "Mark stale after (seconds of poster silence)",
                    value: sent.value("stale_after_secs", Some(&s.stale_after_secs)),
                    help: Some(
                        "At least 3600 (one hour), at most 2592000 (30 days). Default 604800 (7 days).",
                    ),
                    error: sent.error("stale_after_secs"),
                    range: Some(IDLE_RANGE)
                )
            </fieldset>
            save_bar(notice: state.notice_for(TICKET_FORM))
        </form>
    }
    .boxed())
}

#[component]
pub(super) async fn suggestions(
    guild_id: &str,
    settings: &SupportSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let s = settings;
    let sent = state.sent(SUGGESTION_FORM);

    Ok(view! {
        <form
            id=(SUGGESTION_FORM)
            method="post"
            action=(SUGGESTIONS.href(guild_id))
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = sent.summary() {
                form_summary(form: SUGGESTION_FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Suggestions"</legend>
                <p class="page-lead">
                    "A suggestion is posted to the review channel once its \u{1F44D} minus \u{1F44E} count reaches the promote threshold, and removed again if it falls to or below the demote threshold. Tune both to your server size."
                </p>
                select_row(
                    form: SUGGESTION_FORM,
                    name: "suggestions_channel_id",
                    label: "Suggestions channel",
                    selected: sent.value(
                        "suggestions_channel_id",
                        s.suggestions_channel_id.as_deref(),
                    ),
                    options: channels(lists, TEXT_KINDS),
                    error: sent.error("suggestions_channel_id")
                )
                select_row(
                    form: SUGGESTION_FORM,
                    name: "review_channel_id",
                    label: "Review channel",
                    selected: sent.value(
                        "review_channel_id",
                        s.review_channel_id.as_deref(),
                    ),
                    options: channels(lists, TEXT_KINDS),
                    error: sent.error("review_channel_id")
                )
                text_row(
                    form: SUGGESTION_FORM,
                    name: "promote_threshold",
                    label: "Promote at net upvotes",
                    value: sent.value("promote_threshold", Some(&s.promote_threshold)),
                    error: sent.error("promote_threshold"),
                    range: Some(Range { min: 0, max: None })
                )
                text_row(
                    form: SUGGESTION_FORM,
                    name: "demote_threshold",
                    label: "Demote at or below",
                    value: sent.value("demote_threshold", Some(&s.demote_threshold)),
                    help: Some(
                        "Must stay below the promote threshold; a higher value is saved as one below it.",
                    ),
                    error: sent.error("demote_threshold"),
                    input_type: "number",
                    step: Some("1")
                )
            </fieldset>
            save_bar(notice: state.notice_for(SUGGESTION_FORM))
        </form>
    }
    .boxed())
}

#[component]
pub(super) async fn wiki(
    guild_id: &str,
    settings: &FaqSection,
    state: &PageState,
) -> Result<impl View> {
    let s = settings;
    let sent = state.sent(WIKI_FORM);
    let key = state.sent(KEY_FORM);
    let page = WIKI.href(guild_id);
    let key_action = format!("{page}/wiki-key");
    let key_placeholder = if s.wiki_api_key_set {
        "A key is saved - leave blank to keep it"
    } else {
        "eyJhbGciOiJSUzI1NiIs..."
    };

    Ok(view! {
        <form
            id=(WIKI_FORM)
            method="post"
            action=(page.as_str())
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = sent.summary() {
                form_summary(form: WIKI_FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Wiki FAQ"</legend>
                <p class="page-lead">
                    "Backs \"/ticket faq ask\" with a Wiki.js instance. Wiki.js is the only supported wiki - Zayden talks to its GraphQL API and falls back to its source view."
                </p>
                toggle_row(
                    form: WIKI_FORM,
                    name: "enabled",
                    label: "Wiki FAQ",
                    value: sent.flag("enabled", s.enabled),
                    error: sent.error("enabled")
                )
                toggle_row(
                    form: WIKI_FORM,
                    name: "auto_triage",
                    label: "Triage new tickets",
                    value: sent.flag("auto_triage", s.auto_triage),
                    help: Some(
                        "Every new support thread gets an opening embed of suggested articles and follow-up questions. That is two model calls per ticket.",
                    ),
                    error: sent.error("auto_triage")
                )
                toggle_row(
                    form: WIKI_FORM,
                    name: "auto_generate",
                    label: "Write FAQ articles from solved tickets",
                    value: sent.flag("auto_generate", s.auto_generate),
                    help: Some(
                        "\"/ticket solved\" turns the thread into an FAQ article, which goes live immediately and is searchable by \"/ticket faq ask\". Review them under FAQ articles. A ticket that ends without a usable solution produces nothing.",
                    ),
                    error: sent.error("auto_generate")
                )
                text_row(
                    form: WIKI_FORM,
                    name: "wiki_url",
                    label: "Wiki URL",
                    value: sent.value("wiki_url", Some(&s.wiki_url)),
                    help: Some(
                        "Site origin only, starting with http:// or https://, no trailing path. Zayden appends /graphql, /<locale>/ and /s/<locale>/ itself - pointing this at the GraphQL endpoint breaks page reads and article links.",
                    ),
                    error: sent.error("wiki_url"),
                    input_type: "url",
                    placeholder: Some("https://wiki.example.com")
                )
                text_row(
                    form: WIKI_FORM,
                    name: "wiki_locale",
                    label: "Locale",
                    value: sent.value("wiki_locale", Some(&s.wiki_locale)),
                    help: Some(
                        "Letters and hyphens, such as en or pt-br. Blank means en.",
                    ),
                    error: sent.error("wiki_locale")
                )
            </fieldset>
            <fieldset class="settings-section">
                <legend>"Answers"</legend>
                <p class="page-lead">
                    "How \"/ticket faq ask\" searches the wiki and writes its answer."
                </p>
                text_row(
                    form: WIKI_FORM,
                    name: "max_results",
                    label: "Search results to consider",
                    value: sent.value("max_results", Some(&s.max_results)),
                    help: Some("From 1 to 25. Default 5."),
                    error: sent.error("max_results"),
                    range: Some(Range { min: 1, max: Some(25) })
                )
                text_row(
                    form: WIKI_FORM,
                    name: "answer_max_tokens",
                    label: "Answer length (max tokens)",
                    value: sent.value("answer_max_tokens", Some(&s.answer_max_tokens)),
                    help: Some("From 64 to 4096. Default 500."),
                    error: sent.error("answer_max_tokens"),
                    range: Some(Range { min: 64, max: Some(4096) })
                )
                text_row(
                    form: WIKI_FORM,
                    name: "answer_temperature",
                    label: "Answer temperature",
                    value: sent.value("answer_temperature", Some(&s.answer_temperature)),
                    help: Some("From 0 to 2. Lower is more literal. Default 0.2."),
                    error: sent.error("answer_temperature"),
                    range: Some(Range { min: 0, max: Some(2) }),
                    step: Some("0.1")
                )
            </fieldset>
            save_bar(notice: state.notice_for(WIKI_FORM))
        </form>
        <form
            id=(KEY_FORM)
            method="post"
            action=(key_action.as_str())
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = key.summary() {
                form_summary(form: KEY_FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"API key"</legend>
                <p class="page-lead">
                    "The key needs a group with \"read:source\" so Zayden can read page Markdown. Wiki.js also gates its GraphQL page-source queries behind \"manage:pages\"; without either grant the command still answers with matching article links, but cannot summarise them."
                </p>
                text_row(
                    form: KEY_FORM,
                    name: "wiki_api_key",
                    label: "Wiki API key",
                    value: "",
                    help: Some(
                        "A Wiki.js API key. Its group needs read:pages, plus manage:pages or read:source to read page content. A saved key is never sent back to the browser, so leaving this blank keeps it.",
                    ),
                    error: key.error("wiki_api_key"),
                    input_type: "password",
                    placeholder: Some(key_placeholder)
                )
                if s.wiki_api_key_set {
                    toggle_row(
                        form: KEY_FORM,
                        name: "keep_wiki_api_key",
                        label: "Saved API key",
                        value: key.flag("keep_wiki_api_key", true),
                        on_label: "Keep",
                        off_label: "Remove",
                        error: key.error("keep_wiki_api_key")
                    )
                } else {
                    <input type="hidden" name="keep_wiki_api_key" value="true">
                }
            </fieldset>
            save_bar(label: "Save key", notice: state.notice_for(KEY_FORM))
        </form>
    }
    .boxed())
}
