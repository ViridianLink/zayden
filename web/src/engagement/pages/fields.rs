use topcoat::Result;
use topcoat::view::{View, component, view};

use super::action::path_segment;
use super::state::plain;
use crate::auth::AuthError;
use crate::components::error_panel::ErrorAction;
use crate::components::field_row::{describedby, field_row};
use crate::components::icons::{Icon, icon};
use crate::components::pickers::SelectOption;
use crate::engagement::EngagementError;
use crate::guild::GuildError;

#[must_use]
pub(super) fn field_id(form: &str, name: &str) -> String {
    format!("{form}-{}", name.replace('_', "-"))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Constraints {
    pub(super) min: Option<i32>,
    pub(super) max: Option<i32>,
    pub(super) max_length: Option<usize>,
    pub(super) numeric: bool,
    pub(super) required: bool,
}

#[component]
pub(super) async fn text_row(
    form: &str,
    name: &str,
    label: &str,
    value: &str,
    #[default] help: Option<&str>,
    #[default] error: Option<&str>,
    #[default] constraints: Constraints,
    #[default("text")] input_type: &str,
    #[default] placeholder: Option<&str>,
    #[default] pattern: Option<&str>,
) -> Result<impl View> {
    let id = field_id(form, name);
    let ranged = constraints.min.is_some() || constraints.max.is_some();
    let input_type = if ranged { "number" } else { input_type };
    let pattern = pattern.or_else(|| constraints.numeric.then_some("[0-9]*"));

    Ok(view! {
        field_row(
            id: &id,
            label: label,
            help: help,
            error: error,
            <input
                class="input"
                id=(id.as_str())
                type=(input_type)
                name=(name)
                value=(value)
                min=(constraints.min.map(|min| min.to_string()))
                max=(constraints.max.map(|max| max.to_string()))
                step=(ranged.then_some("1"))
                inputmode=(constraints.numeric.then_some("numeric"))
                pattern=(pattern)
                placeholder=(placeholder)
                required=(constraints.required)
                maxlength=(constraints.max_length)
                autocomplete="off"
                aria-describedby=(describedby(&id, help.is_some(), error.is_some()))
                aria-invalid=(error.map(|_| "true"))
            >
        )
    })
}

#[component]
pub(super) async fn select_row(
    form: &str,
    name: &str,
    label: &str,
    selected: &str,
    options: &[SelectOption],
    #[default] help: Option<&str>,
    #[default] error: Option<&str>,
    #[default] required: bool,
    #[default("(not set)")] empty: &str,
) -> Result<impl View> {
    let id = field_id(form, name);
    let has_selected = !selected.is_empty();
    let known = options.iter().any(|option| option.value == selected);
    let fallback = (has_selected && !known).then(|| format!("Unknown ({selected})"));

    Ok(view! {
        field_row(
            id: &id,
            label: label,
            help: help,
            error: error,
            <div class="select">
                <select
                    class="input"
                    id=(id.as_str())
                    name=(name)
                    required=(required)
                    aria-describedby=(describedby(&id, help.is_some(), error.is_some()))
                    aria-invalid=(error.map(|_| "true"))
                >
                    <option value="" selected=(!has_selected)>(empty)</option>
                    if let Some(text) = fallback {
                        <option value=(selected) selected="">(text)</option>
                    }
                    #[key(index)]
                    for (index, option) in options.iter().enumerate() {
                        <option
                            value=(option.value.as_str())
                            selected=(option.value == selected)
                        >
                            (option.label.as_str())
                        </option>
                    }
                </select>
                <span class="select-chevron">icon(name: Icon::ChevronDown)</span>
            </div>
        )
    })
}

#[component]
pub(super) async fn form_summary(
    form: &str,
    message: &str,
    #[default("Not saved")] outcome: &str,
) -> Result<impl View> {
    Ok(view! {
        <div
            class="error"
            id=(format!("{form}-summary"))
            role="alert"
            tabindex="-1"
            autofocus=""
        >
            (format!("{outcome}: {message}"))
        </div>
    })
}

pub(super) fn load_problem(
    guild_id: &str,
    retry: &str,
    error: &EngagementError,
) -> (String, Vec<ErrorAction>) {
    let back = ErrorAction::new("Back to servers", "/guilds");
    let auth = if let EngagementError::Auth(auth)
    | EngagementError::Guild(GuildError::Auth(auth)) = error
    {
        Some(auth)
    } else {
        None
    };

    if auth == Some(&AuthError::BotNotInGuild) {
        let invite = format!("/invite?guild={}", path_segment(guild_id));
        ("Zayden isn't in this server yet.".to_owned(), vec![
            ErrorAction::new("Add Zayden to this server", &invite).external(),
            back,
        ])
    } else if auth == Some(&AuthError::InvalidGuildId) {
        ("That address doesn't name a Discord server.".to_owned(), vec![back])
    } else if error.is_denied() {
        (
            "You need Manage Server in this server to open this page.".to_owned(),
            vec![back],
        )
    } else {
        (format!("Something went wrong: {}", plain(&error.to_string())), vec![
            ErrorAction::new("Try again", retry),
            back,
        ])
    }
}

#[component]
pub(super) async fn load_error(
    title: &str,
    message: &str,
    actions: &[ErrorAction],
) -> Result<impl View> {
    Ok(view! {
        <section class="error-panel" role="alert" aria-labelledby="load-error-title">
            <h2 class="error-title" id="load-error-title">(title)</h2>
            <p class="error-text">(message)</p>
            <div class="error-actions">
                #[key(index)]
                for (index, action) in actions.iter().enumerate() {
                    let class = if index == 0 {
                        "btn btn-primary"
                    } else {
                        "btn btn-secondary"
                    };
                    let rel = action.external.then_some("external");
                    <a href=(action.href.as_str()) rel=(rel) class=(class)>
                        (action.label.as_str())
                    </a>
                }
            </div>
        </section>
    })
}
