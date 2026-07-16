use leptos::prelude::*;
use twilight_model::channel::ChannelType;

use super::icons::Icon;
use crate::dto::{ChannelInfo, RoleInfo};

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
    options: Vec<SelectOption>,
) -> impl IntoView {
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
    channels: Vec<ChannelInfo>,
    #[prop(default = &[])] kinds: &'static [ChannelType],
) -> impl IntoView {
    let options = channels
        .into_iter()
        .filter(|c| kinds.is_empty() || kinds.contains(&c.kind))
        .map(|c| SelectOption {
            label: format!("{}{}", channel_prefix(c.kind), c.name),
            value: c.id,
        })
        .collect();

    view! { <SelectField label=label name=name selected=selected options=options/> }
}

#[component]
pub(crate) fn RoleSelect(
    label: &'static str,
    name: &'static str,
    selected: String,
    roles: Vec<RoleInfo>,
) -> impl IntoView {
    let options = roles
        .into_iter()
        .map(|r| SelectOption { label: format!("@{}", r.name), value: r.id })
        .collect();

    view! { <SelectField label=label name=name selected=selected options=options/> }
}
