#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{Child, View, attributes, component, view};

use super::alert::{AlertVariant, alert, alert_title};
use super::input::input;
use super::pickers::SelectOption;
use super::select::select;
use super::textarea::textarea;

#[must_use]
pub fn field_id(form: &str, name: &str) -> String {
    format!("{form}-{}", name.replace('_', "-"))
}

#[must_use]
pub fn describedby(id: &str, has_help: bool, has_error: bool) -> Option<String> {
    let ids: Vec<String> = [(has_help, "help"), (has_error, "error")]
        .into_iter()
        .filter(|(present, _)| *present)
        .map(|(_, suffix)| format!("{id}-{suffix}"))
        .collect();

    (!ids.is_empty()).then(|| ids.join(" "))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub min: i32,
    pub max: Option<i32>,
}

#[component]
pub async fn field_row(
    id: &str,
    label: &str,
    #[default] help: Option<&str>,
    #[default] error: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class="field-row" data-invalid=(error.map(|_| ""))>
            <label class="field-label" for=(id)>(label)</label>
            (child)
            if let Some(help) = help {
                <p class="m-0 text-[13px] text-text-2" id=(format!("{id}-help"))>
                    (help)
                </p>
            }
            if let Some(error) = error {
                <p class="field-error" id=(format!("{id}-error"))>(error)</p>
            }
        </div>
    })
}

#[component]
pub async fn text_row(
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
    #[default] pattern: Option<&str>,
    #[default] required: bool,
    #[default] max_length: Option<usize>,
) -> Result<impl View> {
    let id = field_id(form, name);
    let input_type = if range.is_some() { "number" } else { input_type };
    let min = range.map(|range| range.min.to_string());
    let max = range.and_then(|range| range.max).map(|max| max.to_string());
    let pattern = pattern.or_else(|| numeric.then_some("[0-9]*"));

    Ok(view! {
        field_row(
            id: &id,
            label: label,
            help: help,
            error: error,
            input(
                attrs: attributes! {
                    id=(id.as_str())
                    type=(input_type)
                    name=(name)
                    value=(value)
                    min=(min)
                    max=(max)
                    step=(step.or_else(|| range.map(|_| "1")))
                    inputmode=(numeric.then_some("numeric"))
                    pattern=(pattern)
                    placeholder=(placeholder)
                    required=(required)
                    maxlength=(max_length)
                    autocomplete="off"
                    aria-describedby=(describedby(&id, help.is_some(), error.is_some()))
                    aria-invalid=(error.map(|_| "true"))
                }
            )
        )
    })
}

#[component]
pub async fn textarea_row(
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
            textarea(
                attrs: attributes! {
                    id=(id.as_str())
                    name=(name)
                    rows=(rows)
                    required=(required)
                    aria-describedby=(describedby(&id, help.is_some(), error.is_some()))
                    aria-invalid=(error.map(|_| "true"))
                },
                (value)
            )
        )
    })
}

#[component]
pub async fn select_row(
    form: &str,
    name: &str,
    label: &str,
    selected: &str,
    options: std::result::Result<Vec<SelectOption>, &str>,
    #[default] help: Option<&str>,
    #[default] error: Option<&str>,
    #[default] required: bool,
    #[default("(not set)")] empty: &str,
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
                    select(
                        attrs: attributes! {
                            id=(id.as_str())
                            name=(name)
                            required=(required)
                            aria-describedby=(describedby(
                                &id,
                                help.is_some(),
                                error.is_some(),
                            ))
                            aria-invalid=(error.map(|_| "true"))
                        },
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
                    )
                )
            }
            Err(reason) => {
                let current = if has_selected {
                    format!("Unchanged ({selected})")
                } else {
                    empty.to_owned()
                };
                field_row(
                    id: &id,
                    label: label,
                    help: Some(reason),
                    error: error,
                    select(
                        attrs: attributes! {
                            id=(id.as_str())
                            aria-describedby=(describedby(&id, true, error.is_some()))
                            disabled=""
                        },
                        <option selected="">(current)</option>
                    )
                    <input type="hidden" name=(name) value=(selected)>
                )
            }
        }
    })
}

#[component]
pub async fn toggle_row(
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
            select(
                attrs: attributes! {
                    id=(id.as_str())
                    name=(name)
                    aria-describedby=(describedby(&id, help.is_some(), error.is_some()))
                    aria-invalid=(error.map(|_| "true"))
                },
                <option value="true" selected=(value)>(on_label)</option>
                <option value="false" selected=(!value)>(off_label)</option>
            )
        )
    })
}

#[component]
pub async fn form_summary(
    form: &str,
    message: &str,
    #[default("Not saved")] outcome: &str,
) -> Result<impl View> {
    Ok(view! {
        alert(
            variant: AlertVariant::Destructive,
            attrs: attributes! {
                class="error"
                id=(format!("{form}-summary"))
                role="alert"
                tabindex="-1"
                autofocus=""
            },
            alert_title((format!("{outcome}: {message}")))
        )
    })
}
