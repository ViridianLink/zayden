use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::not_found;
use topcoat::router::{page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::action::{FormAction, form_action, not_saved, page_href};
use super::fields::{
    Constraints,
    form_summary,
    load_error,
    load_problem,
    select_row,
    text_row,
};
use super::header::page_header;
use super::state::{Done, PageState, rerender, settle};
use crate::auth::{ChannelInfo, RoleInfo};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::data_table::{data_cell, data_row, data_table};
use crate::components::flash::flash;
use crate::components::icons::{Icon, icon};
use crate::components::pickers::{Channel, Role, channel_options, role_options};
use crate::engagement::reaction_roles::{
    AddReactionRoleForm,
    ReactionRolesPage,
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

path_param!(action);

const PAGE: &str = "reaction-roles";
const TITLE: &str = "Reaction roles";
const ADD_FORM: &str = "add-reaction-role";
const LIST_SECTION: &str = "reaction-roles";
const ROLE_ORDER_NOTE: &str = "Zayden's role must be above the roles it assigns. \
                               In Discord, drag it above them under Server \
                               Settings > Roles.";

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
}

impl RoleAction {
    const fn form(self) -> &'static str {
        match self {
            Self::Add => ADD_FORM,
            Self::Remove => "remove-reaction-role",
        }
    }

    const fn section(self) -> &'static str {
        match self {
            Self::Add => ADD_FORM,
            Self::Remove => LIST_SECTION,
        }
    }

    const fn done(self) -> &'static str {
        match self {
            Self::Add => "Reaction role added.",
            Self::Remove => "Reaction role removed.",
        }
    }
}

pub(super) fn action_names() -> Vec<&'static str> {
    RoleAction::ALL.iter().map(|action| action.name()).collect()
}

#[page("/guild/{guild_id}/reaction-roles")]
pub(super) async fn show(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! {
        (state.status())
        reaction_roles_page(guild_id: guild_id, state: &state)
    })
}

/// A post from before the action paths: nothing is applied.
#[page(POST "/guild/{guild_id}/reaction-roles")]
pub(super) async fn legacy_submit(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    not_saved(cx, guild_id, PAGE)?;
    Ok(view! { "" })
}

#[page(POST "/guild/{guild_id}/reaction-roles/{action}")]
pub(super) async fn submit(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let Some(action) = RoleAction::find(path_param::<Action>(cx)) else {
        return Err(not_found().into());
    };

    let page = page_href(guild_id, PAGE);
    let values = pairs.clone();
    let result = save(cx, action, guild_id, pairs).await;
    let failure = settle(cx, action.form(), values, result, &Done {
        page: page.clone(),
        section: Some(action.section()),
        message: action.done(),
    })?;
    Err::<(), _>(rerender(cx, &page, failure))?;
    Ok(view! { "" })
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
    state: &PageState,
) -> Result<impl View> {
    let data = match load_reaction_roles_page(cx, guild_id).await {
        Ok(page) => Ok(page),
        Err(error) => Err(error.redirect_unauthenticated()?),
    };

    Ok(view! {
        <div class="page">
            page_header(
                guild_id: guild_id,
                title: TITLE,
                "Every message \u{2192} emoji \u{2192} role mapping in this server, in one place. Members react to get the role and un-react to lose it."
            )
            flash(notice: state.top_notice())
            match data {
                Err(error) => {
                    let (message, actions) =
                        load_problem(guild_id, &page_href(guild_id, PAGE), &error);
                    if let Some(failure) = state.any_failure() {
                        form_summary(form: "page", message: failure)
                    }
                    load_error(
                        title: "Couldn't load the reaction roles",
                        message: &message,
                        actions: &actions
                    )
                }
                Ok(page) => {
                    add_section(guild_id: guild_id, page: &page, state: state)
                    list_section(guild_id: guild_id, page: &page, state: state)
                }
            }
        </div>
    }
    .boxed())
}

#[component]
async fn add_section(
    guild_id: &str,
    page: &ReactionRolesPage,
    state: &PageState,
) -> Result<impl View> {
    let sent = state.sent(ADD_FORM);
    let channels: Vec<Channel> = page.channels.iter().map(Channel::from).collect();
    let roles: Vec<Role> = page.roles.iter().map(Role::from).collect();
    let channel_choices =
        channel_options(Ok(channels.as_slice()), TEXT_KINDS).unwrap_or_default();
    let role_choices = role_options(Ok(roles.as_slice())).unwrap_or_default();

    Ok(view! {
        <section
            class="settings-section"
            id=(ADD_FORM)
            aria-labelledby="add-reaction-role-title"
        >
            <h2 class="label" id="add-reaction-role-title">"Add a reaction role"</h2>
            <p class="page-lead">
                "Leave the message ID blank and Zayden posts a new panel message in the chosen channel. Give an ID to attach the mapping to a message that already exists - several emoji can share one message."
            </p>
            flash(notice: state.notice_for(ADD_FORM))
            <form
                method="post"
                action=(form_action(guild_id, PAGE, RoleAction::Add))
                data-pending=""
                data-dirty-guard=""
            >
                if let Some(message) = sent.summary() {
                    form_summary(form: ADD_FORM, message: message, outcome: "Not added")
                }
                <input type="hidden" name="guild" value=(guild_id)>
                select_row(
                    form: ADD_FORM,
                    name: "channel_id",
                    label: "Channel",
                    selected: sent.value("channel_id", ""),
                    options: &channel_choices,
                    help: Some("Where the message is, or where Zayden posts the new panel."),
                    error: sent.error("channel_id"),
                    required: true
                )
                text_row(
                    form: ADD_FORM,
                    name: "message_id",
                    label: "Message ID",
                    value: sent.value("message_id", ""),
                    help: Some("Optional. Leave blank to post a new panel message."),
                    error: sent.error("message_id"),
                    constraints: Constraints { numeric: true, ..Constraints::default() }
                )
                text_row(
                    form: ADD_FORM,
                    name: "emoji",
                    label: "Emoji",
                    value: sent.value("emoji", ""),
                    help: Some("A standard emoji such as \u{2705}, or a server emoji written as <:name:id>."),
                    error: sent.error("emoji"),
                    constraints: Constraints { required: true, ..Constraints::default() },
                    placeholder: Some("\u{2705} or <:name:id>")
                )
                select_row(
                    form: ADD_FORM,
                    name: "role_id",
                    label: "Role",
                    selected: sent.value("role_id", ""),
                    options: &role_choices,
                    help: Some(ROLE_ORDER_NOTE),
                    error: sent.error("role_id"),
                    required: true
                )
                <div class="form-actions">
                    <button
                        type="submit"
                        class="btn btn-primary"
                        data-pending-label="Adding\u{2026}"
                    >
                        icon(name: Icon::Plus)
                        "Add reaction role"
                    </button>
                </div>
            </form>
        </section>
    }
    .boxed())
}

#[component]
async fn list_section(
    guild_id: &str,
    page: &ReactionRolesPage,
    state: &PageState,
) -> Result<impl View> {
    let removed = state.sent(RoleAction::Remove.form());

    Ok(view! {
        <section
            class="settings-section"
            id=(LIST_SECTION)
            aria-labelledby="reaction-roles-title"
        >
            <h2 class="label" id="reaction-roles-title">"Mappings"</h2>
            flash(notice: state.notice_for(LIST_SECTION))
            if let Some(message) = removed.summary() {
                form_summary(
                    form: RoleAction::Remove.form(),
                    message: message,
                    outcome: "Not removed"
                )
            }
            if page.mappings.is_empty() {
                <p class="page-lead">
                    "No reaction roles yet - add one above and Zayden will seed the reaction for members to click."
                </p>
            } else {
                data_table(
                    caption: "Reaction roles",
                    columns: &["Channel", "Emoji", "Role", "Message", "Action"],
                    #[key(index)]
                    for (index, mapping) in page.mappings.iter().enumerate() {
                        mapping_row(
                            index: index,
                            guild_id: guild_id,
                            mapping: mapping,
                            channels: &page.channels,
                            roles: &page.roles
                        )
                    }
                )
            }
        </section>
    }
    .boxed())
}

/// A custom emoji's name from `<:name:id>` or `<a:name:id>`.
fn custom_emoji_name(emoji: &str) -> Option<&str> {
    let inner = emoji.strip_prefix('<')?.strip_suffix('>')?;
    let mut parts = inner.split(':');
    let _animated = parts.next()?;
    parts.next().filter(|name| !name.is_empty())
}

#[component]
async fn mapping_row(
    index: usize,
    guild_id: &str,
    mapping: &ReactionRoleInfo,
    channels: &[ChannelInfo],
    roles: &[RoleInfo],
) -> Result<impl View> {
    let channel = channel_label(channels, &mapping.channel_id);
    let role = role_label(roles, &mapping.role_id);
    let link = message_link(guild_id, &mapping.channel_id, &mapping.message_id);
    let action = form_action(guild_id, PAGE, RoleAction::Remove);
    let confirm_id = format!("rr-{index}-remove");
    let emoji_name = custom_emoji_name(&mapping.emoji)
        .map_or_else(|| mapping.emoji.clone(), |name| format!(":{name}:"));
    let confirm_title = format!("Remove the {emoji_name} reaction role for {role}?");

    Ok(view! {
        data_row(
            data_cell(label: "Channel", header: true, (channel))
            data_cell(
                label: "Emoji",
                match custom_emoji_id(&mapping.emoji) {
                    Some(id) => <img
                        class="rr-emoji-img"
                        src=(emoji_image_url(id))
                        alt=(emoji_name.as_str())
                        title=(emoji_name.as_str())
                    >,
                    None => <span class="rr-emoji">(mapping.emoji.as_str())</span>,
                }
            )
            data_cell(label: "Role", (role.as_str()))
            data_cell(
                label: "Message",
                <a class="rr-link" href=(link) rel="external noreferrer" target="_blank">
                    "Open in Discord"
                    icon(name: Icon::ExternalLink)
                </a>
            )
            data_cell(
                label: "Action",
                <form method="post" action=(action) data-pending="">
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
                    confirm_dialog(
                        id: &confirm_id,
                        trigger: "Remove",
                        title: &confirm_title,
                        description: Some(
                            "Reactions already on the message stay, but they stop granting the role.",
                        ),
                        confirm: "Remove reaction role"
                    )
                </form>
            )
        )
    }
    .boxed())
}
