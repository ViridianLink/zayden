use leptos::prelude::*;
use twilight_model::channel::ChannelType;

use super::icons::Icon;
use crate::dto::{ChannelInfo, RoleInfo};

const CHANNELS_UNAVAILABLE: &str = "Couldn't reach Discord; the channel list is \
                                    unavailable. Saving keeps the current value.";

const ROLES_UNAVAILABLE: &str = "Couldn't reach Discord; the role list is \
                                 unavailable. Saving keeps the current value.";

#[derive(Clone)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[component]
pub(crate) fn SelectField(
    label: &'static str,
    name: &'static str,
    selected: String,
    options: Result<Vec<SelectOption>, String>,
) -> impl IntoView {
    match options {
        Ok(options) => picker(label, name, selected, options),
        Err(reason) => locked_picker(label, name, selected, reason),
    }
}

fn picker(
    label: &'static str,
    name: &'static str,
    selected: String,
    options: Vec<SelectOption>,
) -> AnyView {
    let has_selected = !selected.is_empty();
    let known = options.iter().any(|o| o.value == selected);
    let option_views = options
        .into_iter()
        .map(|o| {
            let is_sel = o.value == selected;
            view! { <option value=o.value selected=is_sel>{o.label}</option> }
        })
        .collect_view();
    let fallback = (has_selected && !known).then_some(selected);

    view! {
        <div class="setting-field">
            <label>{label}</label>
            <div class="select">
                <select name=name>
                    <option value="" selected=!has_selected>"(not set)"</option>
                    {fallback.map(|id| {
                        let text = format!("Unknown ({id})");
                        view! { <option value=id selected=true>{text}</option> }
                    })}
                    {option_views}
                </select>
                <span class="select-chevron"><Icon name="chevron-down"/></span>
            </div>
        </div>
    }
    .into_any()
}

// The hidden input carries `selected` in the disabled select's place, so a save
// made while the list is missing round-trips the stored value instead of null.
fn locked_picker(
    label: &'static str,
    name: &'static str,
    selected: String,
    reason: String,
) -> AnyView {
    let current = if selected.is_empty() {
        "(not set)".to_owned()
    } else {
        format!("Unchanged ({selected})")
    };

    view! {
        <div class="setting-field">
            <label>{label}</label>
            <div class="select">
                <select disabled=true>
                    <option selected=true>{current}</option>
                </select>
                <span class="select-chevron"><Icon name="chevron-down"/></span>
            </div>
            <input type="hidden" name=name value=selected/>
            <p class="field-hint field-warning">{reason}</p>
        </div>
    }
    .into_any()
}

fn unavailable(reason: &str, detail: &str) -> String {
    format!("{reason} ({detail})")
}

const fn channel_prefix(kind: ChannelType) -> &'static str {
    match kind {
        ChannelType::GuildVoice => "\u{1F50A} ",
        ChannelType::GuildCategory => "\u{25B8} ",
        ChannelType::GuildAnnouncement => "\u{1F4E2} ",
        ChannelType::GuildStageVoice => "\u{1F3A4} ",
        ChannelType::GuildForum => "\u{1F4AC} ",
        ChannelType::GuildText
        | ChannelType::Private
        | ChannelType::Group
        | ChannelType::AnnouncementThread
        | ChannelType::PublicThread
        | ChannelType::PrivateThread
        | ChannelType::GuildDirectory
        | ChannelType::GuildMedia
        | ChannelType::Unknown(_)
        | _ => "# ",
    }
}

#[component]
pub(crate) fn ChannelSelect(
    label: &'static str,
    name: &'static str,
    selected: String,
    channels: Result<Vec<ChannelInfo>, String>,
    #[prop(default = &[])] kinds: &'static [ChannelType],
) -> impl IntoView {
    let options = channels
        .map(|channels| {
            channels
                .into_iter()
                .filter(|c| kinds.is_empty() || kinds.contains(&c.kind))
                .map(|c| SelectOption {
                    label: format!("{}{}", channel_prefix(c.kind), c.name),
                    value: c.id,
                })
                .collect()
        })
        .map_err(|e| unavailable(CHANNELS_UNAVAILABLE, &e));

    view! { <SelectField label=label name=name selected=selected options=options/> }
}

#[component]
pub(crate) fn RoleSelect(
    label: &'static str,
    name: &'static str,
    selected: String,
    roles: Result<Vec<RoleInfo>, String>,
) -> impl IntoView {
    let options = roles
        .map(|roles| {
            roles
                .into_iter()
                .map(|r| SelectOption { label: format!("@{}", r.name), value: r.id })
                .collect()
        })
        .map_err(|e| unavailable(ROLES_UNAVAILABLE, &e));

    view! { <SelectField label=label name=name selected=selected options=options/> }
}

#[component]
pub(crate) fn ForumTagSelect(
    label: &'static str,
    name: &'static str,
    selected: String,
    channels: Result<Vec<ChannelInfo>, String>,
) -> impl IntoView {
    let options = channels
        .map(|channels| {
            channels
                .into_iter()
                .flat_map(|c| {
                    c.tags.into_iter().map(move |t| SelectOption {
                        label: format!("#{} / {}", c.name, t.name),
                        value: t.id,
                    })
                })
                .collect()
        })
        .map_err(|e| unavailable(CHANNELS_UNAVAILABLE, &e));

    view! { <SelectField label=label name=name selected=selected options=options/> }
}
