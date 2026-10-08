use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::Pane;
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::data_table::{data_cell, data_row, data_table};
use crate::components::flash::flash;
use crate::components::pickers::{Role, SelectOption};
use crate::guild::GuildError;
use crate::guild::dto::{HelperLinkInfo, SupportSection};
use crate::guild::support::{
    AddHelperLinkForm,
    RemoveHelperLinkForm,
    SupportRoleForm,
    add_helper_link,
    add_support_role,
    remove_helper_link,
    remove_support_role,
};
use crate::settings::fields::{
    form_summary,
    roles as role_choices,
    select_row,
    text_row,
};
use crate::settings::state::{Done, Failure, PageState, plain, settle};
use crate::settings::{
    Lists,
    Page,
    ROLE_ORDER_NOTE,
    ensure_path_guild,
    settings_page,
};
use crate::shell::GuildId;

const PAGE: Page = Page::Support(Pane::Roles);

const ROLES_SECTION: &str = "support-roles";
const LINKS_SECTION: &str = "helper-links";
const ADD_ROLE: &str = "add-role";
const REMOVE_ROLE: &str = "remove-role";
const ADD_LINK: &str = "add-link";
const REMOVE_LINK: &str = "remove-link";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(roles_page)
        .page(add_role)
        .page(remove_role)
        .page(add_link)
        .page(remove_link)
}

#[page("/guild/{guild_id}/support/roles")]
async fn roles_page(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

fn act(
    cx: &Cx,
    form: &'static str,
    section: &'static str,
    message: &str,
    values: Vec<(String, String)>,
    result: std::result::Result<(), GuildError>,
) -> Result<Failure> {
    let guild_id: &str = path_param::<GuildId>(cx);
    settle(cx, form, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(section),
        message,
    })
}

#[page(POST "/guild/{guild_id}/support/roles/add-role")]
async fn add_role(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = SupportRoleForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        add_support_role(cx, &form).await
    }
    .await;
    let failure =
        act(cx, ADD_ROLE, ROLES_SECTION, "Support role added.", values, result)?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/support/roles/remove-role")]
async fn remove_role(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = SupportRoleForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        remove_support_role(cx, &form).await
    }
    .await;
    let failure = act(
        cx,
        REMOVE_ROLE,
        ROLES_SECTION,
        "Support role removed.",
        values,
        result,
    )?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/support/roles/add-link")]
async fn add_link(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = AddHelperLinkForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        add_helper_link(cx, &form).await
    }
    .await;
    let failure =
        act(cx, ADD_LINK, LINKS_SECTION, "Helper link saved.", values, result)?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/support/roles/remove-link")]
async fn remove_link(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = RemoveHelperLinkForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        remove_helper_link(cx, &form).await
    }
    .await;
    let failure =
        act(cx, REMOVE_LINK, LINKS_SECTION, "Helper link removed.", values, result)?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

struct RoleRow {
    id: String,
    name: String,
}

#[component]
pub(super) async fn pane(
    guild_id: &str,
    settings: &SupportSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    Ok(view! {
        roles_section(
            guild_id: guild_id,
            support_roles: &settings.support_roles,
            lists: lists,
            state: state
        )
        links_section(
            guild_id: guild_id,
            helper_links: &settings.helper_links,
            state: state
        )
    }
    .boxed())
}

#[component]
async fn roles_section(
    guild_id: &str,
    support_roles: &std::result::Result<Vec<String>, String>,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let known: &[Role] = lists.roles().unwrap_or_default();
    let configured = support_roles.as_deref().unwrap_or_default();
    let rows: Vec<RoleRow> = configured
        .iter()
        .map(|id| RoleRow {
            id: id.clone(),
            name: known.iter().find(|role| role.id == *id).map_or_else(
                || format!("@unknown ({id})"),
                |role| format!("@{}", role.name),
            ),
        })
        .collect();
    let choices = role_choices(lists).map(|options| {
        options
            .into_iter()
            .filter(|option| !configured.contains(&option.value))
            .collect::<Vec<SelectOption>>()
    });
    let page = PAGE.href(guild_id);
    let add_action = format!("{page}/{ADD_ROLE}");
    let remove_action = format!("{page}/{REMOVE_ROLE}");
    let added = state.sent(ADD_ROLE);
    let removed = state.sent(REMOVE_ROLE);

    Ok(view! {
        <section
            class="settings-section"
            id=(ROLES_SECTION)
            aria-labelledby="support-roles-title"
        >
            <h2 class="label" id="support-roles-title">"Support roles"</h2>
            <p class="page-lead">
                "One list, two jobs: these roles are pinged in every new ticket thread, and holding one is what makes somebody a helper - for idle reminders, for donation credit, and for the reminder buttons. With none set, Zayden falls back to pinging the server owner when a ticket opens."
            </p>
            <p class="field-hint">(ROLE_ORDER_NOTE)</p>
            flash(notice: state.notice_for(ROLES_SECTION))
            <form
                id=(ADD_ROLE)
                method="post"
                action=(add_action.as_str())
                data-pending=""
            >
                if let Some(message) = added.summary() {
                    form_summary(form: ADD_ROLE, message: message, outcome: "Not added")
                }
                <input type="hidden" name="guild" value=(guild_id)>
                select_row(
                    form: ADD_ROLE,
                    name: "role_id",
                    label: "Add a support role",
                    selected: added.value("role_id", None),
                    options: choices,
                    error: added.error("role_id")
                )
                <div class="form-actions">
                    <button
                        type="submit"
                        class="btn btn-secondary"
                        data-pending-label="Adding\u{2026}"
                    >
                        "Add role"
                    </button>
                </div>
            </form>
            if let Some(message) = removed.summary() {
                form_summary(
                    form: REMOVE_ROLE,
                    message: message,
                    outcome: "Not removed"
                )
            }
            match support_roles {
                Err(reason) => <p class="warning" role="alert">
                    (format!("Couldn't load the support roles: {}", plain(reason)))
                </p>,
                Ok(_) => {
                    if rows.is_empty() {
                        <p class="page-lead">"No support roles yet."</p>
                    } else {
                        data_table(
                            caption: "Support roles",
                            columns: &["Role", "Action"],
                            #[key(row.id.as_str())]
                            for row in &rows {
                                let confirm_id = format!("role-{}-remove", row.id);
                                let confirm_title = format!(
                                    "Remove support role {}?",
                                    row.name,
                                );
                                data_row(
                                    data_cell(label: "Role", header: true, (row.name.as_str()))
                                    data_cell(
                                        label: "Action",
                                        <form
                                            method="post"
                                            action=(remove_action.as_str())
                                            data-pending=""
                                        >
                                            <input type="hidden" name="guild" value=(guild_id)>
                                            <input
                                                type="hidden"
                                                name="role_id"
                                                value=(row.id.as_str())
                                            >
                                            confirm_dialog(
                                                id: &confirm_id,
                                                trigger: "Remove",
                                                title: &confirm_title,
                                                description: Some(
                                                    "Its members stop being pinged for new tickets and stop counting as helpers.",
                                                ),
                                                confirm: "Remove role"
                                            )
                                        </form>
                                    )
                                )
                            }
                        )
                    }
                }
            }
        </section>
    }
    .boxed())
}

#[component]
async fn links_section(
    guild_id: &str,
    helper_links: &std::result::Result<Vec<HelperLinkInfo>, String>,
    state: &PageState,
) -> Result<impl View> {
    let page = PAGE.href(guild_id);
    let add_action = format!("{page}/{ADD_LINK}");
    let remove_action = format!("{page}/{REMOVE_LINK}");
    let added = state.sent(ADD_LINK);
    let removed = state.sent(REMOVE_LINK);

    Ok(view! {
        <section
            class="settings-section"
            id=(LINKS_SECTION)
            aria-labelledby="helper-links-title"
        >
            <h2 class="label" id="helper-links-title">"Helper donation links"</h2>
            <p class="page-lead">
                "When a post is solved, anyone with a support role who posted in it and has a link here gets credited in a follow-up message."
            </p>
            flash(notice: state.notice_for(LINKS_SECTION))
            <form
                id=(ADD_LINK)
                method="post"
                action=(add_action.as_str())
                data-pending=""
            >
                if let Some(message) = added.summary() {
                    form_summary(form: ADD_LINK, message: message, outcome: "Not saved")
                }
                <input type="hidden" name="guild" value=(guild_id)>
                text_row(
                    form: ADD_LINK,
                    name: "user_id",
                    label: "Helper user ID",
                    value: added.value("user_id", None),
                    help: Some(
                        "The helper's Discord user ID. Adding a link for someone who has one replaces it.",
                    ),
                    error: added.error("user_id"),
                    numeric: true,
                    required: true
                )
                text_row(
                    form: ADD_LINK,
                    name: "link",
                    label: "Donation link",
                    value: added.value("link", None),
                    help: Some("An http:// or https:// address, up to 200 characters."),
                    error: added.error("link"),
                    input_type: "url",
                    required: true,
                    max_length: Some(200)
                )
                <div class="form-actions">
                    <button
                        type="submit"
                        class="btn btn-secondary"
                        data-pending-label="Saving\u{2026}"
                    >
                        "Save link"
                    </button>
                </div>
            </form>
            if let Some(message) = removed.summary() {
                form_summary(
                    form: REMOVE_LINK,
                    message: message,
                    outcome: "Not removed"
                )
            }
            match helper_links {
                Err(reason) => <p class="warning" role="alert">
                    (format!("Couldn't load the helper links: {}", plain(reason)))
                </p>,
                Ok(links) => {
                    if links.is_empty() {
                        <p class="page-lead">"No helper links yet."</p>
                    } else {
                        data_table(
                            caption: "Helper donation links",
                            columns: &["Helper", "Link", "Action"],
                            #[key(link.user_id.as_str())]
                            for link in links {
                                let confirm_id = format!("link-{}-remove", link.user_id);
                                let confirm_title = format!(
                                    "Remove the donation link of {}?",
                                    link.name,
                                );
                                data_row(
                                    data_cell(
                                        label: "Helper",
                                        header: true,
                                        (link.name.as_str())
                                    )
                                    data_cell(
                                        label: "Link",
                                        <a href=(link.link.as_str()) rel="external">
                                            (link.link.as_str())
                                        </a>
                                    )
                                    data_cell(
                                        label: "Action",
                                        <form
                                            method="post"
                                            action=(remove_action.as_str())
                                            data-pending=""
                                        >
                                            <input type="hidden" name="guild" value=(guild_id)>
                                            <input
                                                type="hidden"
                                                name="user_id"
                                                value=(link.user_id.as_str())
                                            >
                                            confirm_dialog(
                                                id: &confirm_id,
                                                trigger: "Remove",
                                                title: &confirm_title,
                                                description: Some(
                                                    "They stop being credited when a post they helped with is solved.",
                                                ),
                                                confirm: "Remove link"
                                            )
                                        </form>
                                    )
                                )
                            }
                        )
                    }
                }
            }
        </section>
    }
    .boxed())
}
