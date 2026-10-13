use topcoat::Result;
use topcoat::context::Cx;
use topcoat::view::{View, ViewExt, component, view};

use super::fields::roles as role_choices;
use super::state::{Done, PageState, settle};
use super::{Lists, Page, ROLE_ORDER_NOTE};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::data_table::{data_cell, data_row, data_table};
use crate::components::field_row::{form_summary, select_row};
use crate::components::flash::flash;
use crate::components::pickers::{Role, SelectOption};
use crate::guild::GuildError;
use crate::guild::announce::MAX_PING_ROLES;

pub(super) const PING_ROLES_SECTION: &str = "ping-roles";
pub(super) const ADD_PING_ROLE: &str = "add-ping-role";
pub(super) const REMOVE_PING_ROLE: &str = "remove-ping-role";

pub(super) fn settle_ping_role(
    cx: &Cx,
    page: Page,
    guild_id: &str,
    form: &'static str,
    values: Vec<(String, String)>,
    result: std::result::Result<(), GuildError>,
) -> Result<PageState> {
    let message =
        if form == ADD_PING_ROLE { "Role added." } else { "Role removed." };
    let failure = settle(cx, form, values, result, &Done {
        page: page.href(guild_id),
        section: Some(PING_ROLES_SECTION),
        message,
    })?;

    Ok(PageState::failed(failure))
}

struct RoleRow {
    id: String,
    name: String,
}

#[component]
pub(super) async fn ping_roles_section(
    guild_id: &str,
    page: Page,
    configured: &[String],
    has_channel: bool,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let known: &[Role] = lists.roles().unwrap_or_default();
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
    let href = page.href(guild_id);
    let add_action = format!("{href}/{ADD_PING_ROLE}");
    let remove_action = format!("{href}/{REMOVE_PING_ROLE}");
    let added = state.sent(ADD_PING_ROLE);
    let removed = state.sent(REMOVE_PING_ROLE);
    let full = configured.len() >= MAX_PING_ROLES;

    Ok(view! {
        <section
            class="settings-section"
            id=(PING_ROLES_SECTION)
            aria-labelledby="ping-roles-title"
        >
            <h2 class="label" id="ping-roles-title">"Roles to ping"</h2>
            <p class="page-lead">
                (format!(
                    "Up to {MAX_PING_ROLES} roles are mentioned where the message has {{roles}}, or at its start when it doesn't.",
                ))
            </p>
            <p class="field-hint">
                "Roles that aren't set as mentionable only ping when Zayden has the Mention @everyone, @here and All Roles permission."
            </p>
            <p class="field-hint">(ROLE_ORDER_NOTE)</p>
            flash(notice: state.notice_for(PING_ROLES_SECTION))
            if !has_channel {
                <p class="page-lead">
                    "Choose an announcement channel and save it before adding roles."
                </p>
            } else if full {
                <p class="page-lead">
                    (format!(
                        "All {MAX_PING_ROLES} role slots are in use. Remove one to add another.",
                    ))
                </p>
            } else {
                <form
                    id=(ADD_PING_ROLE)
                    method="post"
                    action=(add_action.as_str())
                    data-pending=""
                >
                    if let Some(message) = added.summary() {
                        form_summary(
                            form: ADD_PING_ROLE,
                            message: message,
                            outcome: "Not added"
                        )
                    }
                    <input type="hidden" name="guild" value=(guild_id)>
                    select_row(
                        form: ADD_PING_ROLE,
                        name: "role_id",
                        label: "Add a role to ping",
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
            }
            if let Some(message) = removed.summary() {
                form_summary(
                    form: REMOVE_PING_ROLE,
                    message: message,
                    outcome: "Not removed"
                )
            }
            if rows.is_empty() {
                <p class="page-lead">"No roles are pinged."</p>
            } else {
                data_table(
                    caption: "Roles to ping",
                    columns: &["Role", "Action"],
                    #[key(row.id.as_str())]
                    for row in &rows {
                        let confirm_id = format!("ping-role-{}-remove", row.id);
                        let confirm_title = format!("Stop pinging {}?", row.name);
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
                                            "New announcements stop mentioning this role.",
                                        ),
                                        confirm: "Remove role"
                                    )
                                </form>
                            )
                        )
                    }
                )
            }
        </section>
    }
    .boxed())
}
