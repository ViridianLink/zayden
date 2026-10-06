use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::{not_found, see_other};
use topcoat::router::{page, path_param};
use topcoat::view::{View, component, view};

use super::action::{
    FormAction,
    Submitted,
    feedback,
    flagged,
    form_action,
    loaded,
    requested,
};
use crate::auth::{ChannelInfo, RoleInfo};
use crate::components::confirm::confirm_button;
use crate::components::icons::{Icon, icon};
use crate::components::select::{Channel, Role, channel_select, role_select};
use crate::components::settings::{save_feedback, setting_field};
use crate::engagement::reaction_roles::{
    AddReactionRoleForm,
    RemoveReactionRoleForm,
    add_reaction_role,
    load_reaction_roles_page,
    remove_reaction_role,
};
use crate::engagement::{
    EngagementError,
    ReactionRoleInfo,
    TEXT_KINDS,
    channel_label,
    custom_emoji_id,
    emoji_image_url,
    message_link,
    role_label,
};
use crate::shell::GuildId;

const PAGE: &str = "reaction-roles";

#[derive(Clone, Copy, PartialEq, Eq)]
enum RoleAction {
    Add,
    Remove,
}

impl FormAction for RoleAction {
    const ALL: &'static [Self] = &[Self::Add, Self::Remove];

    fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Remove => "remove",
        }
    }

    fn flag(self) -> Option<&'static str> {
        match self {
            Self::Add => Some("added"),
            Self::Remove => Some("removed"),
        }
    }
}

#[page("/guild/{guild_id}/reaction-roles")]
pub(super) async fn show(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { reaction_roles_page(guild_id: guild_id, submitted: flagged(cx)) })
}

#[page(POST "/guild/{guild_id}/reaction-roles")]
pub(super) async fn submit(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let Some(action) = requested::<RoleAction>(cx) else {
        return Err(not_found().into());
    };

    let submitted = Submitted::new(action, save(cx, action, guild_id, pairs).await)?;
    if let Some(location) = submitted.success_location(guild_id, PAGE) {
        return Err(see_other(location).into());
    }
    let status = submitted.status();

    Ok(view! {
        (status)
        reaction_roles_page(guild_id: guild_id, submitted: Some(submitted))
    })
}

async fn save(
    cx: &Cx,
    action: RoleAction,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), EngagementError> {
    match action {
        RoleAction::Add => {
            let form = AddReactionRoleForm::from_pairs(pairs)?;
            form.ensure_path_guild(guild_id)?;
            add_reaction_role(cx, &form).await
        },
        RoleAction::Remove => {
            let form = RemoveReactionRoleForm::from_pairs(pairs)?;
            form.ensure_path_guild(guild_id)?;
            remove_reaction_role(cx, &form).await
        },
    }
}

#[component]
async fn reaction_roles_page(
    cx: &Cx,
    guild_id: &str,
    #[default] submitted: Option<Submitted<RoleAction>>,
) -> Result<impl View> {
    let data = loaded(load_reaction_roles_page(cx, guild_id).await)?;

    Ok(view! {
        <div class="page">
            <div class="page-header">
                <div>
                    <h1>"Reaction Roles"</h1>
                    <p class="page-lead">
                        "Every message \u{2192} emoji \u{2192} role mapping in this server, in one place. Members react to get the role and un-react to lose it."
                    </p>
                </div>
            </div>
            match data {
                Err(error) => <p class="error">
                    "Failed to load reaction roles: "
                    (error)
                </p>,
                Ok(page) => {
                    let removed = feedback(submitted.as_ref(), RoleAction::Remove);
                    let added = feedback(submitted.as_ref(), RoleAction::Add);
                    if let Some(outcome) = removed {
                        save_feedback(outcome: outcome)
                    }
                    mapping_table(
                        guild_id: guild_id,
                        mappings: &page.mappings,
                        channels: &page.channels,
                        roles: &page.roles
                    )
                    add_mapping(
                        guild_id: guild_id,
                        channels: &page.channels,
                        roles: &page.roles,
                        outcome: added
                    )
                }
            }
        </div>
    })
}

#[component]
async fn mapping_table(
    guild_id: &str,
    mappings: &[ReactionRoleInfo],
    channels: &[ChannelInfo],
    roles: &[RoleInfo],
) -> Result<impl View> {
    Ok(view! {
        if mappings.is_empty() {
            <div class="empty">
                "No reaction roles yet - add one below and Zayden will seed the reaction for members to click."
            </div>
        } else {
            <div class="rr-table">
                <div class="rr-row rr-head">
                    <span>"Channel"</span>
                    <span>"Emoji"</span>
                    <span>"Role"</span>
                    <span></span>
                    <span></span>
                </div>
                #[key(index)]
                for (index, mapping) in mappings.iter().enumerate() {
                    mapping_row(
                        guild_id: guild_id,
                        mapping: mapping,
                        channels: channels,
                        roles: roles
                    )
                }
            </div>
        }
    })
}

#[component]
async fn mapping_row(
    guild_id: &str,
    mapping: &ReactionRoleInfo,
    channels: &[ChannelInfo],
    roles: &[RoleInfo],
) -> Result<impl View> {
    let channel = channel_label(channels, &mapping.channel_id);
    let role = role_label(roles, &mapping.role_id);
    let link = message_link(guild_id, &mapping.channel_id, &mapping.message_id);
    let action = form_action(guild_id, PAGE, RoleAction::Remove);

    Ok(view! {
        <div class="rr-row">
            <span class="rr-channel">(channel)</span>
            <span class="rr-cell">
                match custom_emoji_id(&mapping.emoji) {
                    Some(id) => <img
                        class="rr-emoji-img"
                        src=(emoji_image_url(id))
                        alt=""
                    >,
                    None => <span class="rr-emoji">(mapping.emoji.as_str())</span>,
                }
            </span>
            <span class="rr-role">(role)</span>
            <a class="rr-link" href=(link) rel="external noreferrer" target="_blank">
                "Message"
                icon(name: Icon::ExternalLink)
            </a>
            <form class="rr-remove" method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                <input
                    type="hidden"
                    name="channel_id"
                    value=(mapping.channel_id.as_str())
                >
                <input
                    type="hidden"
                    name="message_id"
                    value=(mapping.message_id.as_str())
                >
                <input type="hidden" name="emoji" value=(mapping.emoji.as_str())>
                confirm_button(
                    label: "Remove",
                    prompt: "Reactions already on the message stay, but they stop granting the role.",
                    confirm: "Remove mapping",
                    class: "btn btn-ghost"
                )
            </form>
        </div>
    })
}

#[component]
async fn add_mapping(
    guild_id: &str,
    channels: &[ChannelInfo],
    roles: &[RoleInfo],
    outcome: Option<std::result::Result<(), &str>>,
) -> Result<impl View> {
    let channels: Vec<Channel> = channels.iter().map(Channel::from).collect();
    let roles: Vec<Role> = roles.iter().map(Role::from).collect();
    let action = form_action(guild_id, PAGE, RoleAction::Add);

    Ok(view! {
        <fieldset class="settings-section">
            <legend>
                icon(name: Icon::Plus)
                "Add a mapping"
            </legend>
            if let Some(result) = outcome {
                save_feedback(outcome: result)
            }
            <form method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                channel_select(
                    label: "Channel",
                    name: "channel_id",
                    selected: "",
                    channels: Ok(channels.as_slice()),
                    kinds: TEXT_KINDS
                )
                setting_field(
                    label: "Message ID (blank posts a new panel)",
                    name: "message_id",
                    value: ""
                )
                <div class="setting-field">
                    <label>"Emoji"</label>
                    <input
                        class="input"
                        type="text"
                        name="emoji"
                        placeholder="\u{2705} or <:name:id>"
                    >
                </div>
                role_select(
                    label: "Role",
                    name: "role_id",
                    selected: "",
                    roles: Ok(roles.as_slice())
                )
                <div class="form-actions">
                    <button type="submit" class="btn btn-primary">"Add mapping"</button>
                </div>
            </form>
            <p class="page-lead">
                "Leave the message ID blank and Zayden posts a new panel message in the chosen channel. Give an ID to attach the mapping to a message that already exists - several emoji can share one message."
            </p>
        </fieldset>
    })
}
