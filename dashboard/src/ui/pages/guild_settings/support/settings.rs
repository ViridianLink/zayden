use leptos::form::ActionForm;
use leptos::prelude::*;

use super::super::{TEXT_KINDS, sel};
use crate::dto::{ChannelInfo, GuildSettings, HelperLinkInfo, RoleInfo};
use crate::server::guild::{
    AddHelperLink,
    AddSupportRole,
    RemoveHelperLink,
    RemoveSupportRole,
    SaveFaqSettings,
    SaveFaqTuning,
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
    settings: GuildSettings,
    support_roles: Vec<String>,
    helper_links: Vec<HelperLinkInfo>,
    channels: Vec<ChannelInfo>,
    roles: Vec<RoleInfo>,
    add: ServerAction<AddSupportRole>,
    remove: ServerAction<RemoveSupportRole>,
    add_link: ServerAction<AddHelperLink>,
    remove_link: ServerAction<RemoveHelperLink>,
) -> impl IntoView {
    let save_support = ServerAction::<SaveSupportSettings>::new();
    let result = save_support.value();
    let save_suggestions = ServerAction::<SaveSuggestionsSettings>::new();
    let suggestions_result = save_suggestions.value();

    let faq_settings = settings.clone();
    let s = settings;
    let gid = guild_id.clone();
    let suggestions_gid = guild_id.clone();
    let faq_gid = guild_id.clone();
    let suggestions_channels = channels.clone();

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
                <ToggleField
                    label="Idle Reminders"
                    name="idle_enabled"
                    value=s.support_idle_enabled
                />
                <SettingField
                    label="Remind after (seconds of silence)"
                    name="idle_after_secs"
                    value=s.support_idle_after_secs
                    hint="Minimum one hour. Default 172800 (48 hours)."
                />
                <SaveButton/>
            </ActionForm>
            <p class="page-lead">
                "\"/ticket solved\" applies the solved tag when the support "
                "channel is a forum, and otherwise renames the thread. Archive "
                "after 0 seconds to close immediately, or -1 to leave the post "
                "open."
            </p>
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
                    value=s.suggestions_promote_threshold
                />
                <SettingField
                    label="Demote at or below"
                    name="demote_threshold"
                    value=s.suggestions_demote_threshold
                    pattern="-?[0-9]*"
                />
                <SaveButton/>
            </ActionForm>
            <p class="page-lead">
                "A suggestion is posted to the review channel once "
                "its \u{1F44D} minus \u{1F44E} count reaches the promote "
                "threshold, and removed again if it falls to or below "
                "the demote threshold. Tune both to your server size "
                "- demote must stay below promote."
            </p>
            <FaqField guild_id=faq_gid settings=faq_settings/>
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
fn FaqField(guild_id: String, settings: GuildSettings) -> impl IntoView {
    let save_faq = ServerAction::<SaveFaqSettings>::new();
    let result = save_faq.value();
    let save_tuning = ServerAction::<SaveFaqTuning>::new();
    let tuning_result = save_tuning.value();
    let s = settings;
    let tuning_gid = guild_id.clone();

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
                <ToggleField label="Wiki FAQ" name="enabled" value=s.faq_enabled/>
                <ToggleField
                    label="Triage New Tickets"
                    name="auto_triage"
                    value=s.faq_auto_triage
                />
                <ToggleField
                    label="Write FAQ Articles From Solved Tickets"
                    name="auto_generate"
                    value=s.faq_auto_generate
                />
                <SettingField
                    label="Wiki URL"
                    name="wiki_url"
                    value=s.faq_wiki_url
                    pattern=".*"
                    placeholder="https://wiki.example.com"
                    hint="Site origin only, no trailing path. Zayden appends \
                          /graphql, /<locale>/ and /s/<locale>/ itself - pointing \
                          this at the GraphQL endpoint breaks page reads and \
                          article links."
                />
                <SettingField
                    label="Wiki API Key"
                    name="wiki_api_key"
                    value=s.faq_wiki_api_key
                    pattern=".*"
                    placeholder="eyJhbGciOiJSUzI1NiIs..."
                    hint="A Wiki.js API key. Its group needs read:pages, plus \
                          manage:pages or read:source to read page content."
                />
                <SettingField
                    label="Locale"
                    name="wiki_locale"
                    value=s.faq_wiki_locale
                    pattern="[a-zA-Z-]*"
                />
                <SaveButton/>
            </ActionForm>
            {move || tuning_result.get().map(save_feedback)}
            <ActionForm action=save_tuning>
                <input type="hidden" name="guild" value=tuning_gid/>
                <SettingField
                    label="Search results to consider"
                    name="max_results"
                    value=s.faq_max_results
                />
                <SettingField
                    label="Answer length (max tokens)"
                    name="answer_max_tokens"
                    value=s.faq_answer_max_tokens
                />
                <SettingField
                    label="Answer temperature"
                    name="answer_temperature"
                    value=s.faq_answer_temperature
                    pattern="[0-9.]*"
                />
                <SaveButton/>
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
    helper_links: Vec<HelperLinkInfo>,
    add: ServerAction<AddHelperLink>,
    remove: ServerAction<RemoveHelperLink>,
) -> impl IntoView {
    let add_result = add.value();
    let remove_result = remove.value();

    let chips = helper_links
        .into_iter()
        .map(|l| {
            let gid = guild_id.clone();
            let label = format!("{} \u{2192} {}", l.name, l.link);

            view! {
                <ActionForm action=remove attr:class="chip">
                    <input type="hidden" name="guild" value=gid/>
                    <input type="hidden" name="user_id" value=l.user_id/>
                    <span class="chip-label">{label}</span>
                    <button type="submit" class="chip-remove" title="Remove">
                        <Icon name="x"/>
                    </button>
                </ActionForm>
            }
        })
        .collect_view();

    view! {
        <div class="setting-field">
            <label>"Helper Donation Links"</label>
            <p class="page-lead">
                "When a post is solved, anyone with a support role who posted "
                "in it and has a link here gets credited in a follow-up message."
            </p>
            <div class="chip-list">{chips}</div>
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
                <button type="submit" class="btn btn-ghost">"Add link"</button>
            </ActionForm>
        </div>
    }
}

#[component]
fn SupportRoleField(
    guild_id: String,
    support_roles: Vec<String>,
    roles: Vec<RoleInfo>,
    add: ServerAction<AddSupportRole>,
    remove: ServerAction<RemoveSupportRole>,
) -> impl IntoView {
    let add_result = add.value();
    let remove_result = remove.value();

    let unconfigured = roles
        .iter()
        .filter(|r| !support_roles.contains(&r.id))
        .cloned()
        .collect::<Vec<_>>();

    let chips = support_roles
        .into_iter()
        .map(|id| {
            let name = roles.iter().find(|r| r.id == id).map_or_else(
                || format!("@unknown ({id})"),
                |r| format!("@{}", r.name),
            );
            let gid = guild_id.clone();

            view! {
                <ActionForm action=remove attr:class="chip">
                    <input type="hidden" name="guild" value=gid/>
                    <input type="hidden" name="role_id" value=id/>
                    <span class="chip-label">{name}</span>
                    <button type="submit" class="chip-remove" title="Remove">
                        <Icon name="x"/>
                    </button>
                </ActionForm>
            }
        })
        .collect_view();

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
            <div class="chip-list">{chips}</div>
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
                <button type="submit" class="btn btn-ghost">"Add role"</button>
            </ActionForm>
        </div>
    }
}
