use leptos::form::ActionForm;
use leptos::prelude::*;
use twilight_model::channel::ChannelType;

use crate::dto::{ChannelInfo, GuildSettings};
use crate::server::guild::{CreateTempVoiceCreatorChannel, SaveTempVoiceSettings};
use crate::ui::components::icons::Icon;
use crate::ui::components::select::ChannelSelect;
use crate::ui::components::settings::{SaveButton, create_feedback, save_feedback};

#[component]
pub(crate) fn TempVoiceTab(
    guild_id: String,
    settings: GuildSettings,
    channels: Vec<ChannelInfo>,
    create: ServerAction<CreateTempVoiceCreatorChannel>,
) -> impl IntoView {
    let save_temp_voice = ServerAction::<SaveTempVoiceSettings>::new();
    let save_result = save_temp_voice.value();
    let create_result = create.value();

    let GuildSettings { temp_voice_category, temp_voice_creator_channel, .. } =
        settings;
    let save_channels = channels.clone();
    // The category doubles as the target for the "create one for me" form.
    let save_category = temp_voice_category.unwrap_or_default();
    let create_category = save_category.clone();
    let creator = temp_voice_creator_channel.unwrap_or_default();
    let gid = guild_id.clone();

    view! {
        <fieldset class="settings-section">
            <legend><Icon name="mic"/>"Temp Voice"</legend>
            {move || save_result.get().map(save_feedback)}
            <ActionForm action=save_temp_voice>
                <input type="hidden" name="guild" value=gid/>
                <ChannelSelect
                    label="Category"
                    name="temp_voice_category"
                    selected=save_category
                    channels=save_channels.clone()
                    kinds=&[ChannelType::GuildCategory]
                />
                <ChannelSelect
                    label="Creator Channel"
                    name="temp_voice_creator_channel"
                    selected=creator
                    channels=save_channels
                    kinds=&[ChannelType::GuildVoice]
                />
                <SaveButton/>
            </ActionForm>
            <p class="page-lead">
                "No creator channel yet? Zayden can make one for you "
                "and point the settings above at it."
            </p>
            {move || create_result.get().map(create_feedback)}
            <ActionForm action=create>
                <input type="hidden" name="guild" value=guild_id/>
                <ChannelSelect
                    label="Create Creator Channel In"
                    name="temp_voice_category"
                    selected=create_category
                    channels=channels
                    kinds=&[ChannelType::GuildCategory]
                />
                <div class="form-actions">
                    <button type="submit" class="btn btn-secondary">
                        "Create Creator Channel"
                    </button>
                </div>
            </ActionForm>
        </fieldset>
    }
}
