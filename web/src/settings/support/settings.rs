use topcoat::Result;
use topcoat::runtime::Signal;
use topcoat::view::{View, ViewExt, component, view};

use super::{
    ADD_LINK,
    ADD_ROLE,
    IDLE,
    REMOVE_LINK,
    REMOVE_ROLE,
    REMOVE_ROLE_QUERY,
    STALE,
    SUGGESTIONS,
    TICKETS,
    WIKI,
    WIKI_KEY,
    WIKI_TUNING,
    form_action,
};
use crate::components::icons::{Icon, icon};
use crate::components::select::{
    Role,
    channel_select,
    forum_tag_select,
    role_select,
};
use crate::components::settings::{
    save_button,
    save_feedback,
    setting_field,
    toggle_field,
};
use crate::guild::dto::{FaqSection, HelperLinkInfo, SupportSection};
use crate::settings::{Lists, Submission, TEXT_KINDS, flag, shown};

#[component]
pub(super) async fn pane(
    guild_id: &str,
    settings: &SupportSection,
    lists: &Lists,
    submission: Option<&Submission>,
    faq_open: &Signal<bool>,
) -> Result<impl View> {
    let action = form_action(guild_id);
    let s = settings;

    Ok(view! {
        <fieldset class="settings-section" :hidden=$(faq_open.get())>
            tickets_form(
                guild_id: guild_id,
                action: &action,
                settings: s,
                lists: lists,
                submission: submission
            )
            idle_form(
                guild_id: guild_id,
                action: &action,
                settings: s,
                submission: submission
            )
            stale_form(
                guild_id: guild_id,
                action: &action,
                settings: s,
                lists: lists,
                submission: submission
            )
            turn_notes()
            suggestions_form(
                guild_id: guild_id,
                action: &action,
                settings: s,
                lists: lists,
                submission: submission
            )
            wiki_field(
                guild_id: guild_id,
                action: &action,
                settings: &s.faq,
                submission: submission
            )
            support_role_field(
                guild_id: guild_id,
                action: &action,
                support_roles: &s.support_roles,
                lists: lists,
                submission: submission
            )
            helper_link_field(
                guild_id: guild_id,
                action: &action,
                helper_links: &s.helper_links,
                submission: submission
            )
        </fieldset>
    }
    .boxed())
}

#[component]
async fn tickets_form(
    guild_id: &str,
    action: &str,
    settings: &SupportSection,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let s = settings;
    let tickets = Submission::of(submission, TICKETS);

    Ok(view! {
        if let Some(submitted) = tickets {
            save_feedback(outcome: submitted.outcome())
        }
        <form method="post" action=(action) data-pending="">
            <input type="hidden" name="guild" value=(guild_id)>
            channel_select(
                label: "Support Channel",
                name: "support_channel_id",
                selected: shown(
                    tickets,
                    "support_channel_id",
                    s.support_channel_id.as_deref(),
                ),
                channels: lists.channels(),
                kinds: TEXT_KINDS
            )
            forum_tag_select(
                label: "Solved Tag",
                name: "solved_tag_id",
                selected: shown(tickets, "solved_tag_id", s.solved_tag_id.as_deref()),
                channels: lists.channels()
            )
            forum_tag_select(
                label: "Closed Tag",
                name: "closed_tag_id",
                selected: shown(tickets, "closed_tag_id", s.closed_tag_id.as_deref()),
                channels: lists.channels()
            )
            setting_field(
                label: "Archive solved posts after (seconds)",
                name: "solved_archive_secs",
                value: shown(
                    tickets,
                    "solved_archive_secs",
                    Some(s.solved_archive_secs.as_str()),
                ),
                pattern: "-?[0-9]*"
            )
            save_button()
        </form>
        <p class="page-lead">
            "\"/ticket solved\" applies the solved tag when the support channel is a forum, and otherwise renames the thread. Archive after 0 seconds to close immediately, or -1 to leave the post open."
        </p>
    }
    .boxed())
}

#[component]
async fn idle_form(
    guild_id: &str,
    action: &str,
    settings: &SupportSection,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let s = settings;
    let idle = Submission::of(submission, IDLE);

    Ok(view! {
        if let Some(submitted) = idle {
            save_feedback(outcome: submitted.outcome())
        }
        <form method="post" action=(action) data-pending="">
            <input type="hidden" name="guild" value=(guild_id)>
            toggle_field(
                label: "Idle Reminders",
                name: "idle_enabled",
                value: flag(idle, "idle_enabled", s.idle_enabled)
            )
            setting_field(
                label: "Remind after (seconds of silence)",
                name: "idle_after_secs",
                value: shown(idle, "idle_after_secs", Some(s.idle_after_secs.as_str())),
                hint: Some("Minimum one hour. Default 172800 (48 hours).")
            )
            toggle_field(
                label: "Auto-close Abandoned Posts",
                name: "idle_close_enabled",
                value: flag(idle, "idle_close_enabled", s.idle_close_enabled)
            )
            setting_field(
                label: "Close after (seconds without a reply to the reminder)",
                name: "idle_close_after_secs",
                value: shown(
                    idle,
                    "idle_close_after_secs",
                    Some(s.idle_close_after_secs.as_str()),
                ),
                hint: Some("Minimum one hour. Default 86400 (24 hours).")
            )
            save_button()
        </form>
    }
    .boxed())
}

#[component]
async fn stale_form(
    guild_id: &str,
    action: &str,
    settings: &SupportSection,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let s = settings;
    let stale = Submission::of(submission, STALE);

    Ok(view! {
        if let Some(submitted) = stale {
            save_feedback(outcome: submitted.outcome())
        }
        <form method="post" action=(action) data-pending="">
            <input type="hidden" name="guild" value=(guild_id)>
            toggle_field(
                label: "Mark Quiet Posts Stale",
                name: "stale_enabled",
                value: flag(stale, "stale_enabled", s.stale_enabled)
            )
            forum_tag_select(
                label: "Stale Tag",
                name: "stale_tag_id",
                selected: shown(stale, "stale_tag_id", s.stale_tag_id.as_deref()),
                channels: lists.channels()
            )
            setting_field(
                label: "Mark stale after (seconds of poster silence)",
                name: "stale_after_secs",
                value: shown(
                    stale,
                    "stale_after_secs",
                    Some(s.stale_after_secs.as_str()),
                ),
                hint: Some("Minimum one hour. Default 604800 (7 days).")
            )
            save_button()
        </form>
    }
    .boxed())
}

#[component]
async fn turn_notes() -> Result<impl View> {
    Ok(view! {
        <p class="page-lead">
            "Idle reminders watch whose turn it is. If a helper spoke last and the poster has gone quiet for the interval above, the poster is nudged with \"Solved\" and \"Still need help\" buttons. If the poster spoke last, the helper who replied is nudged - or the support roles, if nobody has answered yet. Each side is reminded once per turn, and never again until somebody posts."
        </p>
        <p class="page-lead">
            "Auto-close needs idle reminders switched on, because it acts on a reminder nobody answered. Only posts waiting on the person who opened them are closed - a post waiting on the support team is left alone however long it sits, since that is the team's backlog and not an abandoned ticket. Closing applies the closed tag, posts a note to the poster and archives the post. Any reply at all cancels it."
        </p>
        <p class="page-lead">
            "The stale tag is a quieter signal than closing: a post the poster has left unanswered for the interval above is tagged so the team can see at a glance what has gone cold. It needs a forum tag to apply, follows the same staff-side exemption as auto-close, and comes off again by itself as soon as anybody posts."
        </p>
        <p class="page-lead">
            "A support role only gets notified if it is mentionable. Role mentions in private ticket threads mostly do not notify at all, since Discord does not pull role members into a private thread. A reminder also un-archives a post Discord had already archived, which is usually the point."
        </p>
    })
}

#[component]
async fn suggestions_form(
    guild_id: &str,
    action: &str,
    settings: &SupportSection,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let s = settings;
    let suggestions = Submission::of(submission, SUGGESTIONS);

    Ok(view! {
        if let Some(submitted) = suggestions {
            save_feedback(outcome: submitted.outcome())
        }
        <form method="post" action=(action) data-pending="">
            <input type="hidden" name="guild" value=(guild_id)>
            channel_select(
                label: "Suggestions Channel",
                name: "suggestions_channel_id",
                selected: shown(
                    suggestions,
                    "suggestions_channel_id",
                    s.suggestions_channel_id.as_deref(),
                ),
                channels: lists.channels(),
                kinds: TEXT_KINDS
            )
            channel_select(
                label: "Review Channel",
                name: "review_channel_id",
                selected: shown(
                    suggestions,
                    "review_channel_id",
                    s.review_channel_id.as_deref(),
                ),
                channels: lists.channels(),
                kinds: TEXT_KINDS
            )
            setting_field(
                label: "Promote at net upvotes",
                name: "promote_threshold",
                value: shown(
                    suggestions,
                    "promote_threshold",
                    Some(s.promote_threshold.as_str()),
                )
            )
            setting_field(
                label: "Demote at or below",
                name: "demote_threshold",
                value: shown(
                    suggestions,
                    "demote_threshold",
                    Some(s.demote_threshold.as_str()),
                ),
                pattern: "-?[0-9]*"
            )
            save_button()
        </form>
        <p class="page-lead">
            "A suggestion is posted to the review channel once its \u{1F44D} minus \u{1F44E} count reaches the promote threshold, and removed again if it falls to or below the demote threshold. Tune both to your server size - demote must stay below promote."
        </p>
    }
    .boxed())
}

#[component]
async fn wiki_field(
    guild_id: &str,
    action: &str,
    settings: &FaqSection,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let s = settings;
    let wiki = Submission::of(submission, WIKI);
    let key = Submission::of(submission, WIKI_KEY);
    let tuning = Submission::of(submission, WIKI_TUNING);
    let key_placeholder = if s.wiki_api_key_set {
        "A key is saved - leave blank to keep it"
    } else {
        "eyJhbGciOiJSUzI1NiIs..."
    };

    Ok(view! {
        <div class="setting-field">
            <label>"Wiki FAQ"</label>
            <p class="page-lead">
                "Backs \"/ticket faq ask\" with a Wiki.js instance. Wiki.js is the only supported wiki - Zayden talks to its GraphQL API and falls back to its source view."
            </p>
            if let Some(submitted) = wiki {
                save_feedback(outcome: submitted.outcome())
            }
            <form method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                toggle_field(
                    label: "Wiki FAQ",
                    name: "enabled",
                    value: flag(wiki, "enabled", s.enabled)
                )
                toggle_field(
                    label: "Triage New Tickets",
                    name: "auto_triage",
                    value: flag(wiki, "auto_triage", s.auto_triage)
                )
                toggle_field(
                    label: "Write FAQ Articles From Solved Tickets",
                    name: "auto_generate",
                    value: flag(wiki, "auto_generate", s.auto_generate)
                )
                setting_field(
                    label: "Wiki URL",
                    name: "wiki_url",
                    value: shown(wiki, "wiki_url", Some(s.wiki_url.as_str())),
                    pattern: ".*",
                    placeholder: "https://wiki.example.com",
                    hint: Some(
                        "Site origin only, no trailing path. Zayden appends /graphql, /<locale>/ and /s/<locale>/ itself - pointing this at the GraphQL endpoint breaks page reads and article links.",
                    )
                )
                setting_field(
                    label: "Locale",
                    name: "wiki_locale",
                    value: shown(wiki, "wiki_locale", Some(s.wiki_locale.as_str())),
                    pattern: "[a-zA-Z-]*"
                )
                save_button()
            </form>
            if let Some(submitted) = key {
                save_feedback(outcome: submitted.outcome())
            }
            <form method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                setting_field(
                    label: "Wiki API Key",
                    name: "wiki_api_key",
                    value: "",
                    pattern: ".*",
                    placeholder: key_placeholder,
                    hint: Some(
                        "A Wiki.js API key. Its group needs read:pages, plus manage:pages or read:source to read page content. A saved key is never sent back to the browser, so leaving this blank keeps it.",
                    ),
                    input_type: "password"
                )
                if s.wiki_api_key_set {
                    toggle_field(
                        label: "Saved API Key",
                        name: "keep_wiki_api_key",
                        value: flag(key, "keep_wiki_api_key", true),
                        on_label: "Keep",
                        off_label: "Remove"
                    )
                } else {
                    <input type="hidden" name="keep_wiki_api_key" value="true">
                }
                save_button()
            </form>
            if let Some(submitted) = tuning {
                save_feedback(outcome: submitted.outcome())
            }
            <form method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                setting_field(
                    label: "Search results to consider",
                    name: "max_results",
                    value: shown(tuning, "max_results", Some(s.max_results.as_str()))
                )
                setting_field(
                    label: "Answer length (max tokens)",
                    name: "answer_max_tokens",
                    value: shown(
                        tuning,
                        "answer_max_tokens",
                        Some(s.answer_max_tokens.as_str()),
                    )
                )
                setting_field(
                    label: "Answer temperature",
                    name: "answer_temperature",
                    value: shown(
                        tuning,
                        "answer_temperature",
                        Some(s.answer_temperature.as_str()),
                    ),
                    pattern: "[0-9.]*"
                )
                save_button()
            </form>
            <p class="page-lead">
                "The API key needs a group with \"read:source\" so Zayden can read page Markdown. Wiki.js also gates its GraphQL page-source queries behind \"manage:pages\"; without either grant the command still answers with matching article links, but cannot summarise them."
            </p>
            <p class="page-lead">
                "With triage on, every new support thread gets an opening embed of suggested articles and follow-up questions. That is two model calls per ticket."
            </p>
            <p class="page-lead">
                "With article writing on, \"/ticket solved\" turns the thread into an FAQ article, which goes live immediately and is searchable by \"/ticket faq ask\". Review them under the FAQ tab. A ticket that ends without a usable solution produces nothing."
            </p>
        </div>
    }
    .boxed())
}

#[component]
async fn chip_feedback(
    submission: Option<&Submission>,
    remove: &str,
    add: &str,
) -> Result<impl View> {
    Ok(view! {
        if let Some(submitted) = Submission::of(submission, remove) {
            save_feedback(outcome: submitted.outcome())
        }
        if let Some(submitted) = Submission::of(submission, add) {
            save_feedback(outcome: submitted.outcome())
        }
    })
}

#[component]
async fn support_role_field(
    guild_id: &str,
    action: &str,
    support_roles: &std::result::Result<Vec<String>, String>,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let known = lists.roles().unwrap_or_default();
    let configured = support_roles.as_deref().unwrap_or_default();
    let unconfigured = lists.roles().map(|roles| {
        roles
            .iter()
            .filter(|role| !configured.contains(&role.id))
            .cloned()
            .collect::<Vec<Role>>()
    });
    let remove_action = format!("{action}?{REMOVE_ROLE_QUERY}");

    Ok(view! {
        <div class="setting-field">
            <label>"Support Roles"</label>
            <p class="page-lead">
                "One list, two jobs: these roles are pinged in every new ticket thread, and holding one is what makes somebody a helper - for idle reminders, for donation credit, and for the reminder buttons. With none set, Zayden falls back to pinging the server owner when a ticket opens."
            </p>
            match support_roles {
                Ok(ids) => <div class="chip-list">
                    #[key(id.as_str())]
                    for id in ids {
                        let name = known
                            .iter()
                            .find(|role| role.id == *id)
                            .map_or_else(
                                || format!("@unknown ({id})"),
                                |role| format!("@{}", role.name),
                            );
                        <form
                            class="chip"
                            method="post"
                            action=(remove_action.as_str())
                            data-pending=""
                        >
                            <input type="hidden" name="guild" value=(guild_id)>
                            <input type="hidden" name="role_id" value=(id.as_str())>
                            <span class="chip-label">(name)</span>
                            <button type="submit" class="chip-remove" title="Remove">
                                icon(name: Icon::X)
                            </button>
                        </form>
                    }
                </div>,
                Err(reason) => <p class="warning">
                    "Couldn't load the support roles: "
                    (reason)
                </p>,
            }
            chip_feedback(submission: submission, remove: REMOVE_ROLE, add: ADD_ROLE)
            <form class="chip-add" method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                role_select(
                    label: "Add a support role",
                    name: "role_id",
                    selected: "",
                    roles: unconfigured.as_ref().map(Vec::as_slice).map_err(|e| *e)
                )
                <button type="submit" class="btn btn-ghost">"Add role"</button>
            </form>
        </div>
    })
}

#[component]
async fn helper_link_field(
    guild_id: &str,
    action: &str,
    helper_links: &std::result::Result<Vec<HelperLinkInfo>, String>,
    submission: Option<&Submission>,
) -> Result<impl View> {
    Ok(view! {
        <div class="setting-field">
            <label>"Helper Donation Links"</label>
            <p class="page-lead">
                "When a post is solved, anyone with a support role who posted in it and has a link here gets credited in a follow-up message."
            </p>
            match helper_links {
                Ok(links) => <div class="chip-list">
                    #[key(link.user_id.as_str())]
                    for link in links {
                        let label = format!("{} \u{2192} {}", link.name, link.link);
                        <form
                            class="chip"
                            method="post"
                            action=(action)
                            data-pending=""
                        >
                            <input type="hidden" name="guild" value=(guild_id)>
                            <input
                                type="hidden"
                                name="user_id"
                                value=(link.user_id.as_str())
                            >
                            <span class="chip-label">(label)</span>
                            <button type="submit" class="chip-remove" title="Remove">
                                icon(name: Icon::X)
                            </button>
                        </form>
                    }
                </div>,
                Err(reason) => <p class="warning">
                    "Couldn't load the helper links: "
                    (reason)
                </p>,
            }
            chip_feedback(submission: submission, remove: REMOVE_LINK, add: ADD_LINK)
            <form class="chip-add" method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                setting_field(label: "Helper user ID", name: "user_id", value: "")
                setting_field(
                    label: "Donation link",
                    name: "link",
                    value: "",
                    pattern: ".*"
                )
                <button type="submit" class="btn btn-ghost">"Add link"</button>
            </form>
        </div>
    })
}
