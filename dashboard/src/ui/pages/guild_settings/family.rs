use leptos::form::ActionForm;
use leptos::prelude::*;

use crate::dto::GuildSettings;
use crate::server::guild::SaveFamilySettings;
use crate::ui::components::icons::Icon;
use crate::ui::components::settings::{SaveButton, SettingField, save_feedback};

#[component]
pub(crate) fn FamilyTab(guild_id: String, settings: GuildSettings) -> impl IntoView {
    let save_family = ServerAction::<SaveFamilySettings>::new();
    let result = save_family.value();

    view! {
        <fieldset class="settings-section">
            <legend><Icon name="heart"/>"Family"</legend>
            {move || result.get().map(save_feedback)}
            <ActionForm action=save_family>
                <input type="hidden" name="guild" value=guild_id/>
                <SettingField
                    label="Max Partners"
                    name="max_partners"
                    value=settings.family_max_partners
                />
                <SaveButton/>
            </ActionForm>
        </fieldset>
    }
}
