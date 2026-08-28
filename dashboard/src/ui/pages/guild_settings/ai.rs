use leptos::form::ActionForm;
use leptos::prelude::*;

use super::{TEXT_KINDS, sel};
use crate::dto::{ChannelInfo, GuildSettings};
use crate::server::guild::SaveAiSettings;
use crate::ui::components::select::ChannelSelect;
use crate::ui::components::settings::{SaveButton, ToggleField, save_feedback};

#[component]
pub(crate) fn AiTab(
    guild_id: String,
    settings: GuildSettings,
    channels: Vec<ChannelInfo>,
) -> impl IntoView {
    let save_ai = ServerAction::<SaveAiSettings>::new();
    let result = save_ai.value();
    let s = settings;

    view! {
        <fieldset class="settings-section">
            {move || result.get().map(save_feedback)}
            <ActionForm action=save_ai>
                <input type="hidden" name="guild" value=guild_id/>
                <ToggleField label="AI Responses" name="enabled" value=s.ai_enabled/>
                <ChannelSelect
                    label="Restrict to Channel"
                    name="channel_id"
                    selected=sel(s.ai_channel_id.as_deref())
                    channels=channels
                    kinds=TEXT_KINDS
                />
                <SaveButton/>
            </ActionForm>
            <p class="page-lead">
                "With AI responses on, Zayden replies in character "
                "whenever someone mentions him. Leave the channel "
                "unset to let him answer anywhere he can see, or "
                "pick one to keep him to a single room."
            </p>
            <p class="page-lead">
                "Every reply costs a model call, so scope this to a "
                "channel you actually want him talking in. The "
                "toggle here is the same switch as the AI Chat card "
                "on the Modules page."
            </p>
        </fieldset>
    }
}
