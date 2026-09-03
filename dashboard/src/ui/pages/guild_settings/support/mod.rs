pub mod faq;
pub mod settings;

use leptos::prelude::*;

use self::faq::FaqArticlesPane;
use self::settings::SupportSettingsPane;
use crate::dto::{ChannelInfo, GuildSettings, HelperLinkInfo, RoleInfo};
use crate::server::guild::{
    AddHelperLink,
    AddSupportRole,
    RemoveHelperLink,
    RemoveSupportRole,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pane {
    Settings,
    Faq,
}

#[component]
pub(crate) fn SupportTab(
    guild_id: String,
    settings: GuildSettings,
    support_roles: Vec<String>,
    helper_links: Vec<HelperLinkInfo>,
    channels: Vec<ChannelInfo>,
    roles: Vec<RoleInfo>,
    add: ServerAction<AddSupportRole>,
    remove: ServerAction<RemoveSupportRole>,
    add_link: ServerAction<AddHelperLink>,
    remove_link: ServerAction<RemoveHelperLink>,
) -> impl IntoView {
    let pane = RwSignal::new(Pane::Settings);
    let faq_guild_id = guild_id.clone();

    view! {
        <div class="segmented" role="tablist">
            <button
                type="button"
                class=move || seg_class(pane.get() == Pane::Settings)
                on:click=move |_| pane.set(Pane::Settings)
            >
                "Settings"
            </button>
            <button
                type="button"
                class=move || seg_class(pane.get() == Pane::Faq)
                on:click=move |_| pane.set(Pane::Faq)
            >
                "FAQ"
            </button>
        </div>
        <Show when=move || pane.get() == Pane::Settings>
            <SupportSettingsPane
                guild_id=guild_id.clone()
                settings=settings.clone()
                support_roles=support_roles.clone()
                helper_links=helper_links.clone()
                channels=channels.clone()
                roles=roles.clone()
                add=add
                remove=remove
                add_link=add_link
                remove_link=remove_link
            />
        </Show>
        <Show when=move || pane.get() == Pane::Faq>
            <FaqArticlesPane guild_id=faq_guild_id.clone()/>
        </Show>
    }
}

const fn seg_class(active: bool) -> &'static str {
    if active { "seg active" } else { "seg" }
}
