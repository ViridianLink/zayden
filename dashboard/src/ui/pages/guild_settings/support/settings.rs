use leptos::form::ActionForm;
use leptos::prelude::*;

use super::super::{TEXT_KINDS, sel};
use crate::dto::{
    ChannelInfo,
    FaqSection,
    HelperLinkInfo,
    RoleInfo,
    SupportSection,
};
use crate::server::guild::{
    AddHelperLink,
    AddSupportRole,
    RemoveHelperLink,
    RemoveSupportRole,
    SaveFaqSettings,
    SaveFaqTuning,
    SaveFaqWikiKey,
    SaveIdleSettings,
    SaveStaleSettings,
    SaveSuggestionsSettings,
    SaveSupportSettings,
};
use crate::ui::components::icons::Icon;
use crate::ui::components::select::{ChannelSelect, ForumTagSelect, RoleSelect};
use crate::ui::components::settings::{
    SaveButton,
    SettingField,
    ToggleField,
    save_feedback,
};

#[component]
pub(crate) fn SupportSettingsPane(
    guild_id: String,
    settings: SupportSection,
    channels: Result<Vec<ChannelInfo>, String>,
    roles: Result<Vec<RoleInfo>, String>,
    add: ServerAction<AddSupportRole>,
    remove: ServerAction<RemoveSupportRole>,
    add_link: ServerAction<AddHelperLink>,
    remove_link: ServerAction<RemoveHelperLink>,
) -> impl IntoView {
    let save_support = ServerAction::<SaveSupportSettings>::new();
    let result = save_support.value();
    let save_idle = ServerAction::<SaveIdleSettings>::new();
    let idle_result = save_idle.value();
    let save_stale = ServerAction::<SaveStaleSettings>::new();
    let stale_result = save_stale.value();
    let save_suggestions = ServerAction::<SaveSuggestionsSettings>::new();
    let suggestions_result = save_suggestions.value();

    let SupportSection { faq, support_roles, helper_links, .. } = settings.clone();
    let s = settings;
    let gid = guild_id.clone();
    let idle_gid = guild_id.clone();
    let stale_gid = guild_id.clone();
    let suggestions_gid = guild_id.clone();
    let faq_gid = guild_id.clone();
    let suggestions_channels = channels.clone();
    let idle_channels = channels.clone();

    view! {
        <fieldset class="settings-section">
            {move || result.get().map(save_feedback)}
            <ActionForm action=save_support>
                <input type="hidden" name="guild" value=gid/>
                <ChannelSelect
                    label="Support Channel"
                    name="support_channel_id"
                    selected=sel(s.support_channel_id.as_deref())
                    channels=channels.clone()
                    kinds=TEXT_KINDS
                />
                <ForumTagSelect
                    label="Solved Tag"
                    name="solved_tag_id"
                    selected=sel(s.solved_tag_id.as_deref())
                    channels=channels.clone()
                />
                <ForumTagSelect
                    label="Closed Tag"
                    name="closed_tag_id"
                    selected=sel(s.closed_tag_id.as_deref())
                    channels=channels
                />
                <SettingField
                    label="Archive solved posts after (seconds)"
                    name="solved_archive_secs"
                    value=s.solved_archive_secs
                    pattern="-?[0-9]*"
                />
                <SaveButton pending=save_support.pending()/>
            </ActionForm>
            <p class="page-lead">
                "\"/ticket solved\" applies the solved tag when the support "
                "channel is a forum, and otherwise renames the thread. Archive "
                "after 0 seconds to close immediately, or -1 to leave the post "
                "open."
            </p>
            {move || idle_result.get().map(save_feedback)}
            <ActionForm action=save_idle>
                <input type="hidden" name="guild" value=idle_gid/>
                <ToggleField
                    label="Idle Reminders"
                    name="idle_enabled"
                    value=s.idle_enabled
                />
                <SettingField
                    label="Remind after (seconds of silence)"
                    name="idle_after_secs"
                    value=s.idle_after_secs
                    hint="Minimum one hour. Default 172800 (48 hours)."
                />
                <ToggleField
                    label="Auto-close Abandoned Posts"
                    name="idle_close_enabled"
                    value=s.idle_close_enabled
                />
                <SettingField
                    label="Close after (seconds without a reply to the reminder)"
                    name="idle_close_after_secs"
                    value=s.idle_close_after_secs
                    hint="Minimum one hour. Default 86400 (24 hours)."
                />
                <SaveButton pending=save_idle.pending()/>
            </ActionForm>
            {move || stale_result.get().map(save_feedback)}
            <ActionForm action=save_stale>
                <input type="hidden" name="guild" value=stale_gid/>
                <ToggleField
                    label="Mark Quiet Posts Stale"
                    name="stale_enabled"
                    value=s.stale_enabled
                />
                <ForumTagSelect
                    label="Stale Tag"
                    name="stale_tag_id"
                    selected=sel(s.stale_tag_id.as_deref())
                    channels=idle_channels
                />
                <SettingField
                    label="Mark stale after (seconds of poster silence)"
                    name="stale_after_secs"
                    value=s.stale_after_secs
                    hint="Minimum one hour. Default 604800 (7 days)."
                />
                <SaveButton pending=save_stale.pending()/>
            </ActionForm>
            <p class="page-lead">
                "Idle reminders watch whose turn it is. If a helper spoke last "
                "and the poster has gone quiet for the interval above, the "
                "poster is nudged with \"Solved\" and \"Still need help\" "
                "buttons. If the poster spoke last, the helper who replied is "
                "nudged - or the support roles, if nobody has answered yet. "
                "Each side is reminded once per turn, and never again until "
                "somebody posts."
            </p>
            <p class="page-lead">
                "Auto-close needs idle reminders switched on, because it acts "
                "on a reminder nobody answered. Only posts waiting on the "
                "person who opened them are closed - a post waiting on the "
                "support team is left alone however long it sits, since that "
                "is the team's backlog and not an abandoned ticket. Closing "
                "applies the closed tag, posts a note to the poster and "
                "archives the post. Any reply at all cancels it."
            </p>
            <p class="page-lead">
                "The stale tag is a quieter signal than closing: a post the "
                "poster has left unanswered for the interval above is tagged "
                "so the team can see at a glance what has gone cold. It needs "
                "a forum tag to apply, follows the same staff-side exemption "
                "as auto-close, and comes off again by itself as soon as "
                "anybody posts."
            </p>
            <p class="page-lead">
                "A support role only gets notified if it is mentionable. Role "
                "mentions in private ticket threads mostly do not notify at "
                "all, since Discord does not pull role members into a private "
                "thread. A reminder also un-archives a post Discord had "
                "already archived, which is usually the point."
            </p>
            {move || suggestions_result.get().map(save_feedback)}
            <ActionForm action=save_suggestions>
                <input type="hidden" name="guild" value=suggestions_gid/>
                <ChannelSelect
                    label="Suggestions Channel"
                    name="suggestions_channel_id"
                    selected=sel(s.suggestions_channel_id.as_deref())
                    channels=suggestions_channels.clone()
                    kinds=TEXT_KINDS
                />
                <ChannelSelect
                    label="Review Channel"
                    name="review_channel_id"
                    selected=sel(s.review_channel_id.as_deref())
                    channels=suggestions_channels
                    kinds=TEXT_KINDS
                />
                <SettingField
                    label="Promote at net upvotes"
                    name="promote_threshold"
                    value=s.promote_threshold
                />
                <SettingField
                    label="Demote at or below"
                    name="demote_threshold"
                    value=s.demote_threshold
                    pattern="-?[0-9]*"
                />
                <SaveButton pending=save_suggestions.pending()/>
            </ActionForm>
            <p class="page-lead">
                "A suggestion is posted to the review channel once "
                "its \u{1F44D} minus \u{1F44E} count reaches the promote "
                "threshold, and removed again if it falls to or below "
                "the demote threshold. Tune both to your server size "
                "- demote must stay below promote."
            </p>
            <FaqField guild_id=faq_gid settings=faq/>
            <SupportRoleField
                guild_id=guild_id.clone()
                support_roles=support_roles
                roles=roles
                add=add
                remove=remove
            />
            <HelperLinkField
                guild_id=guild_id
                helper_links=helper_links
                add=add_link
                remove=remove_link
            />
        </fieldset>
    }
}

#[component]
fn FaqField(guild_id: String, settings: FaqSection) -> impl IntoView {
    let save_faq = ServerAction::<SaveFaqSettings>::new();
    let result = save_faq.value();
    let save_key = ServerAction::<SaveFaqWikiKey>::new();
    let key_result = save_key.value();
    let save_tuning = ServerAction::<SaveFaqTuning>::new();
    let tuning_result = save_tuning.value();
    let s = settings;
    let key_gid = guild_id.clone();
    let tuning_gid = guild_id.clone();
    let key_set = s.wiki_api_key_set;
    let key_placeholder = if key_set {
        "A key is saved - leave blank to keep it"
    } else {
        "eyJhbGciOiJSUzI1NiIs..."
    };

    view! {
        <div class="setting-field">
            <label>"Wiki FAQ"</label>
            <p class="page-lead">
                "Backs \"/ticket faq ask\" with a Wiki.js instance. Wiki.js is the "
                "only supported wiki - Zayden talks to its GraphQL API and falls "
                "back to its source view."
            </p>
            {move || result.get().map(save_feedback)}
            <ActionForm action=save_faq>
                <input type="hidden" name="guild" value=guild_id/>
                <ToggleField label="Wiki FAQ" name="enabled" value=s.enabled/>
                <ToggleField
                    label="Triage New Tickets"
                    name="auto_triage"
                    value=s.auto_triage
                />
                <ToggleField
                    label="Write FAQ Articles From Solved Tickets"
                    name="auto_generate"
                    value=s.auto_generate
                />
                <SettingField
                    label="Wiki URL"
                    name="wiki_url"
                    value=s.wiki_url
                    pattern=".*"
                    placeholder="https://wiki.example.com"
                    hint="Site origin only, no trailing path. Zayden appends \
                          /graphql, /<locale>/ and /s/<locale>/ itself - pointing \
                          this at the GraphQL endpoint breaks page reads and \
                          article links."
                />
                <SettingField
                    label="Locale"
                    name="wiki_locale"
                    value=s.wiki_locale
                    pattern="[a-zA-Z-]*"
                />
                <SaveButton pending=save_faq.pending()/>
            </ActionForm>
            {move || key_result.get().map(save_feedback)}
            <ActionForm action=save_key>
                <input type="hidden" name="guild" value=key_gid/>
                <SettingField
                    label="Wiki API Key"
                    name="wiki_api_key"
                    input_type="password"
                    value=String::new()
                    pattern=".*"
                    placeholder=key_placeholder
                    hint="A Wiki.js API key. Its group needs read:pages, plus \
                          manage:pages or read:source to read page content. A \
                          saved key is never sent back to the browser, so \
                          leaving this blank keeps it."
                />
                {if key_set {
                    view! {
                        <ToggleField
                            label="Saved API Key"
                            name="keep_wiki_api_key"
                            value=true
                            on_label="Keep"
                            off_label="Remove"
                        />
                    }
                    .into_any()
                } else {
                    view! {
                        <input type="hidden" name="keep_wiki_api_key" value="true"/>
                    }
                    .into_any()
                }}
                <SaveButton pending=save_key.pending()/>
            </ActionForm>
            {move || tuning_result.get().map(save_feedback)}
            <ActionForm action=save_tuning>
                <input type="hidden" name="guild" value=tuning_gid/>
                <SettingField
                    label="Search results to consider"
                    name="max_results"
                    value=s.max_results
                />
                <SettingField
                    label="Answer length (max tokens)"
                    name="answer_max_tokens"
                    value=s.answer_max_tokens
                />
                <SettingField
                    label="Answer temperature"
                    name="answer_temperature"
                    value=s.answer_temperature
                    pattern="[0-9.]*"
                />
                <SaveButton pending=save_tuning.pending()/>
            </ActionForm>
            <p class="page-lead">
                "The API key needs a group with \"read:source\" so Zayden can "
                "read page Markdown. Wiki.js also gates its GraphQL page-source "
                "queries behind \"manage:pages\"; without either grant the "
                "command still answers with matching article links, but cannot "
                "summarise them."
            </p>
            <p class="page-lead">
                "With triage on, every new support thread gets an opening embed "
                "of suggested articles and follow-up questions. That is two "
                "model calls per ticket."
            </p>
            <p class="page-lead">
                "With article writing on, \"/ticket solved\" turns the thread "
                "into an FAQ article, which "
                "goes live immediately and is searchable by \"/ticket faq "
                "ask\". Review them under the FAQ tab. A ticket that ends "
                "without a usable solution produces nothing."
            </p>
        </div>
    }
}

#[component]
fn HelperLinkField(
    guild_id: String,
    helper_links: Result<Vec<HelperLinkInfo>, String>,
    add: ServerAction<AddHelperLink>,
    remove: ServerAction<RemoveHelperLink>,
) -> impl IntoView {
    let add_result = add.value();
    let remove_result = remove.value();

    let chips = match helper_links {
        Ok(links) => {
            let views = links
                .into_iter()
                .map(|l| {
                    let gid = guild_id.clone();
                    let label = format!("{} \u{2192} {}", l.name, l.link);

                    view! {
                        <ActionForm action=remove attr:class="chip">
                            <input type="hidden" name="guild" value=gid/>
                            <input type="hidden" name="user_id" value=l.user_id/>
                            <span class="chip-label">{label}</span>
                            <button
                                type="submit"
                                class="chip-remove"
                                title="Remove"
                                disabled=remove.pending()
                            >
                                <Icon name="x"/>
                            </button>
                        </ActionForm>
                    }
                })
                .collect_view();

            view! { <div class="chip-list">{views}</div> }.into_any()
        },
        Err(reason) => view! {
            <p class="warning">"Couldn't load the helper links: " {reason}</p>
        }
        .into_any(),
    };

    view! {
        <div class="setting-field">
            <label>"Helper Donation Links"</label>
            <p class="page-lead">
                "When a post is solved, anyone with a support role who posted "
                "in it and has a link here gets credited in a follow-up message."
            </p>
            {chips}
            {move || remove_result.get().map(save_feedback)}
            {move || add_result.get().map(save_feedback)}
            <ActionForm action=add attr:class="chip-add">
                <input type="hidden" name="guild" value=guild_id/>
                <SettingField
                    label="Helper user ID"
                    name="user_id"
                    value=String::new()
                />
                <SettingField
                    label="Donation link"
                    name="link"
                    value=String::new()
                    pattern=".*"
                />
                <button
                    type="submit"
                    class="btn btn-ghost"
                    disabled=add.pending()
                >
                    "Add link"
                </button>
            </ActionForm>
        </div>
    }
}

#[component]
fn SupportRoleField(
    guild_id: String,
    support_roles: Result<Vec<String>, String>,
    roles: Result<Vec<RoleInfo>, String>,
    add: ServerAction<AddSupportRole>,
    remove: ServerAction<RemoveSupportRole>,
) -> impl IntoView {
    let add_result = add.value();
    let remove_result = remove.value();

    let known = roles.clone().unwrap_or_default();
    let configured = support_roles.clone().unwrap_or_default();
    let unconfigured = roles.map(|roles| {
        roles.into_iter().filter(|r| !configured.contains(&r.id)).collect::<Vec<_>>()
    });

    let chips = match support_roles {
        Ok(ids) => {
            let views = ids
                .into_iter()
                .map(|id| {
                    let name = known.iter().find(|r| r.id == id).map_or_else(
                        || format!("@unknown ({id})"),
                        |r| format!("@{}", r.name),
                    );
                    let gid = guild_id.clone();

                    view! {
                        <ActionForm action=remove attr:class="chip">
                            <input type="hidden" name="guild" value=gid/>
                            <input type="hidden" name="role_id" value=id/>
                            <span class="chip-label">{name}</span>
                            <button
                                type="submit"
                                class="chip-remove"
                                title="Remove"
                                disabled=remove.pending()
                            >
                                <Icon name="x"/>
                            </button>
                        </ActionForm>
                    }
                })
                .collect_view();

            view! { <div class="chip-list">{views}</div> }.into_any()
        },
        Err(reason) => view! {
            <p class="warning">"Couldn't load the support roles: " {reason}</p>
        }
        .into_any(),
    };

    view! {
        <div class="setting-field">
            <label>"Support Roles"</label>
            <p class="page-lead">
                "One list, two jobs: these roles are pinged in every new "
                "ticket thread, and holding one is what makes somebody a "
                "helper - for idle reminders, for donation credit, and for the "
                "reminder buttons. With none set, Zayden falls back to pinging "
                "the server owner when a ticket opens."
            </p>
            {chips}
            {move || remove_result.get().map(save_feedback)}
            {move || add_result.get().map(save_feedback)}
            <ActionForm action=add attr:class="chip-add">
                <input type="hidden" name="guild" value=guild_id/>
                <RoleSelect
                    label="Add a support role"
                    name="role_id"
                    selected=String::new()
                    roles=unconfigured
                />
                <button
                    type="submit"
                    class="btn btn-ghost"
                    disabled=add.pending()
                >
                    "Add role"
                </button>
            </ActionForm>
        </div>
    }
}
