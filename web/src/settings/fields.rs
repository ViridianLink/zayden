use topcoat::Result;
use topcoat::view::{View, component, view};
use twilight_model::channel::ChannelType;

use super::Lists;
use crate::components::field_row::{describedby, field_row};
use crate::components::icons::{Icon, icon};
use crate::components::pickers::{
    SelectOption,
    channel_options,
    forum_tag_options,
    role_options,
};

const CHANNELS_UNAVAILABLE: &str = "Couldn't reach Discord, so the channel list \
                                    is unavailable. Saving keeps the current \
                                    value.";

const ROLES_UNAVAILABLE: &str = "Couldn't reach Discord, so the role list is \
                                 unavailable. Saving keeps the current value.";

#[must_use]
pub(super) fn field_id(form: &str, name: &str) -> String {
    format!("{form}-{}", name.replace('_', "-"))
}

pub(super) fn channels(
    lists: &Lists,
    kinds: &[ChannelType],
) -> std::result::Result<Vec<SelectOption>, &'static str> {
    lists
        .channels()
        .ok()
        .and_then(|channels| channel_options(Ok(channels), kinds).ok())
        .ok_or(CHANNELS_UNAVAILABLE)
}

pub(super) fn roles(
    lists: &Lists,
) -> std::result::Result<Vec<SelectOption>, &'static str> {
    lists
        .roles()
        .ok()
        .and_then(|roles| role_options(Ok(roles)).ok())
        .ok_or(ROLES_UNAVAILABLE)
}

pub(super) fn forum_tags(
    lists: &Lists,
) -> std::result::Result<Vec<SelectOption>, &'static str> {
    lists
        .channels()
        .ok()
        .and_then(|channels| forum_tag_options(Ok(channels)).ok())
        .ok_or(CHANNELS_UNAVAILABLE)
}

#[component]
pub(super) async fn select_row(
    form: &str,
    name: &str,
    label: &str,
    selected: &str,
    options: std::result::Result<Vec<SelectOption>, &str>,
    #[default] help: Option<&str>,
    #[default] error: Option<&str>,
) -> Result<impl View> {
    let id = field_id(form, name);
    let has_selected = !selected.is_empty();

    Ok(view! {
        match options {
            Ok(options) => {
                let known = options.iter().any(|option| option.value == selected);
                let fallback = (has_selected && !known).then(
                    || format!("Unknown ({selected})"),
                );
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
                            aria-describedby=(describedby(
                                &id,
                                help.is_some(),
                                error.is_some(),
                            ))
                            aria-invalid=(error.map(|_| "true"))
                        >
                            <option value="" selected=(!has_selected)>
                                "(not set)"
                            </option>
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
                        <span class="select-chevron">
                            icon(name: Icon::ChevronDown)
                        </span>
                    </div>
                )
            }
            Err(reason) => {
                let current = if has_selected {
                    format!("Unchanged ({selected})")
                } else {
                    "(not set)".to_owned()
                };
                field_row(
                    id: &id,
                    label: label,
                    help: Some(reason),
                    error: error,
                    <div class="select">
                        <select
                            class="input"
                            id=(id.as_str())
                            aria-describedby=(describedby(&id, true, error.is_some()))
                            disabled=""
                        >
                            <option selected="">(current)</option>
                        </select>
                        <span class="select-chevron">
                            icon(name: Icon::ChevronDown)
                        </span>
                    </div>
                    <input type="hidden" name=(name) value=(selected)>
                )
            }
        }
    })
}

#[component]
pub(super) async fn toggle_row(
    form: &str,
    name: &str,
    label: &str,
    value: bool,
    #[default("On")] on_label: &str,
    #[default("Off")] off_label: &str,
    #[default] help: Option<&str>,
    #[default] error: Option<&str>,
) -> Result<impl View> {
    let id = field_id(form, name);

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
                    aria-describedby=(describedby(&id, help.is_some(), error.is_some()))
                    aria-invalid=(error.map(|_| "true"))
                >
                    <option value="true" selected=(value)>(on_label)</option>
                    <option value="false" selected=(!value)>(off_label)</option>
                </select>
                <span class="select-chevron">icon(name: Icon::ChevronDown)</span>
            </div>
        )
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Range {
    pub(super) min: i32,
    pub(super) max: Option<i32>,
}

#[component]
pub(super) async fn text_row(
    form: &str,
    name: &str,
    label: &str,
    value: &str,
    #[default] help: Option<&str>,
    #[default] error: Option<&str>,
    #[default] range: Option<Range>,
    #[default] step: Option<&str>,
    #[default] numeric: bool,
    #[default("text")] input_type: &str,
    #[default] placeholder: Option<&str>,
    #[default] required: bool,
    #[default] max_length: Option<usize>,
) -> Result<impl View> {
    let id = field_id(form, name);
    let input_type = if range.is_some() { "number" } else { input_type };
    let min = range.map(|range| range.min.to_string());
    let max = range.and_then(|range| range.max).map(|max| max.to_string());

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
                min=(min)
                max=(max)
                step=(step.or_else(|| range.map(|_| "1")))
                inputmode=(numeric.then_some("numeric"))
                pattern=(numeric.then_some("[0-9]*"))
                placeholder=(placeholder)
                required=(required)
                maxlength=(max_length)
                autocomplete="off"
                aria-describedby=(describedby(&id, help.is_some(), error.is_some()))
                aria-invalid=(error.map(|_| "true"))
            >
        )
    })
}

#[component]
pub(super) async fn textarea_row(
    form: &str,
    name: &str,
    label: &str,
    value: &str,
    #[default(8)] rows: u32,
    #[default] help: Option<&str>,
    #[default] error: Option<&str>,
    #[default] required: bool,
) -> Result<impl View> {
    let id = field_id(form, name);

    Ok(view! {
        field_row(
            id: &id,
            label: label,
            help: help,
            error: error,
            <textarea
                class="input"
                id=(id.as_str())
                name=(name)
                rows=(rows)
                required=(required)
                aria-describedby=(describedby(&id, help.is_some(), error.is_some()))
                aria-invalid=(error.map(|_| "true"))
            >
                (value)
            </textarea>
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
