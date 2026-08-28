use leptos::form::ActionForm;
use leptos::prelude::*;

use super::{TEXT_KINDS, sel};
use crate::dto::{ChannelInfo, GuildSettings, RoleInfo};
use crate::server::guild::{AddSupportRole, RemoveSupportRole, SaveSupportSettings};
use crate::ui::components::icons::Icon;
use crate::ui::components::select::{ChannelSelect, RoleSelect};
use crate::ui::components::settings::{SaveButton, SettingField, save_feedback};

#[component]
pub(crate) fn SupportTab(
    guild_id: String,
    settings: GuildSettings,
    support_roles: Vec<String>,
    channels: Vec<ChannelInfo>,
    roles: Vec<RoleInfo>,
    add: ServerAction<AddSupportRole>,
    remove: ServerAction<RemoveSupportRole>,
) -> impl IntoView {
    let save_support = ServerAction::<SaveSupportSettings>::new();
    let result = save_support.value();

    let s = settings;
    let gid = guild_id.clone();

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
                <ChannelSelect
                    label="FAQ Channel"
                    name="faq_channel_id"
                    selected=sel(s.faq_channel_id.as_deref())
                    channels=channels.clone()
                    kinds=TEXT_KINDS
                />
                <ChannelSelect
                    label="Suggestions Channel"
                    name="suggestions_channel_id"
                    selected=sel(s.suggestions_channel_id.as_deref())
                    channels=channels.clone()
                    kinds=TEXT_KINDS
                />
                <ChannelSelect
                    label="Review Channel"
                    name="review_channel_id"
                    selected=sel(s.review_channel_id.as_deref())
                    channels=channels
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
            <SupportRoleField
                guild_id=guild_id
                support_roles=support_roles
                roles=roles
                add=add
                remove=remove
            />
        </fieldset>
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
                "Pinged in every new ticket thread. With none set, Zayden falls "
                "back to pinging the server owner."
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
