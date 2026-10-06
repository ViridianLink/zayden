mod channels;
mod cooldowns;
mod images;
mod messages;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::{not_found, see_other};
use topcoat::router::{page, path_param};
use topcoat::view::{View, ViewExt, component, view};

pub use self::channels::channel_section;
use self::cooldowns::cooldown_section;
use self::images::image_section;
use self::messages::messages_section;
use super::action::{
    FormAction,
    Submitted,
    feedback,
    flagged,
    loaded,
    requested,
    typed,
};
use crate::components::settings::save_feedback;
use crate::engagement::EngagementError;
use crate::engagement::greetings::{
    AddGreetingImageForm,
    GreetingChannelForm,
    GreetingsPage,
    RemoveGreetingImageForm,
    SaveGreetingCooldownsForm,
    SaveGreetingMessagesForm,
    add_greeting_channel,
    add_greeting_image,
    load_greetings_page,
    remove_greeting_channel,
    remove_greeting_image,
    save_greeting_cooldowns,
    save_greeting_messages,
};
use crate::shell::GuildId;

const PAGE: &str = "greetings";

#[derive(Clone, Copy, PartialEq, Eq)]
enum GreetingAction {
    SaveMessages,
    SaveCooldowns,
    AddChannel,
    RemoveChannel,
    AddImage,
    RemoveImage,
}

impl FormAction for GreetingAction {
    const ALL: &'static [Self] = &[
        Self::SaveMessages,
        Self::SaveCooldowns,
        Self::AddChannel,
        Self::RemoveChannel,
        Self::AddImage,
        Self::RemoveImage,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::SaveMessages => "save-messages",
            Self::SaveCooldowns => "save-cooldowns",
            Self::AddChannel => "add-channel",
            Self::RemoveChannel => "remove-channel",
            Self::AddImage => "add-image",
            Self::RemoveImage => "remove-image",
        }
    }

    fn flag(self) -> Option<&'static str> {
        match self {
            Self::SaveMessages | Self::SaveCooldowns => None,
            Self::AddChannel => Some("channel_added"),
            Self::RemoveChannel => Some("channel_removed"),
            Self::AddImage => Some("image_added"),
            Self::RemoveImage => Some("image_removed"),
        }
    }
}

#[page("/guild/{guild_id}/greetings")]
pub(super) async fn show(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { greetings_page(guild_id: guild_id, submitted: flagged(cx)) })
}

#[page(POST "/guild/{guild_id}/greetings")]
pub(super) async fn submit(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let Some(action) = requested::<GreetingAction>(cx) else {
        return Err(not_found().into());
    };

    let values = pairs.clone();
    let submitted =
        Submitted::new(action, values, save(cx, action, guild_id, pairs).await)?;
    if let Some(location) = submitted.success_location(guild_id, PAGE) {
        return Err(see_other(location).into());
    }

    let status = submitted.status();

    Ok(view! {
        (status)
        greetings_page(guild_id: guild_id, submitted: Some(submitted))
    })
}

async fn save(
    cx: &Cx,
    action: GreetingAction,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), EngagementError> {
    match action {
        GreetingAction::SaveMessages => {
            let form = SaveGreetingMessagesForm::from_pairs(pairs)?;
            form.ensure_path_guild(guild_id)?;
            save_greeting_messages(cx, &form).await
        },
        GreetingAction::SaveCooldowns => {
            let form = SaveGreetingCooldownsForm::from_pairs(pairs)?;
            form.ensure_path_guild(guild_id)?;
            save_greeting_cooldowns(cx, &form).await
        },
        GreetingAction::AddChannel => {
            let form = GreetingChannelForm::from_pairs(pairs)?;
            form.ensure_path_guild(guild_id)?;
            add_greeting_channel(cx, &form).await
        },
        GreetingAction::RemoveChannel => {
            let form = GreetingChannelForm::from_pairs(pairs)?;
            form.ensure_path_guild(guild_id)?;
            remove_greeting_channel(cx, &form).await
        },
        GreetingAction::AddImage => {
            let form = AddGreetingImageForm::from_pairs(pairs)?;
            form.ensure_path_guild(guild_id)?;
            add_greeting_image(cx, &form).await
        },
        GreetingAction::RemoveImage => {
            let form = RemoveGreetingImageForm::from_pairs(pairs)?;
            form.ensure_path_guild(guild_id)?;
            remove_greeting_image(cx, &form).await
        },
    }
}

#[component]
async fn greetings_page(
    cx: &Cx,
    guild_id: &str,
    #[default] submitted: Option<Submitted<GreetingAction>>,
) -> Result<impl View> {
    let data = loaded(load_greetings_page(cx, guild_id).await)?;

    Ok(view! {
        <div class="page">
            <div class="page-header">
                <div>
                    <h1>"Greetings"</h1>
                    <p class="page-lead">
                        "What Zayden posts for "
                        <code>"/good morning"</code>
                        " and "
                        <code>"/good night"</code>
                        ". Each subcommand replies with one image picked at random from its list, plus the message below if you set one."
                    </p>
                </div>
            </div>
            match data {
                Err(error) => <p class="error">
                    "Failed to load greetings: "
                    (error)
                </p>,
                Ok(page) => greeting_sections(
                    guild_id: guild_id,
                    page: &page,
                    submitted: submitted.as_ref()
                ),
            }
        </div>
    })
}

#[component]
async fn greeting_sections(
    guild_id: &str,
    page: &GreetingsPage,
    submitted: Option<&Submitted<GreetingAction>>,
) -> Result<impl View> {
    Ok(view! {
        messages_section(
            guild_id: guild_id,
            morning: page.view.morning_message.as_str(),
            night: page.view.night_message.as_str(),
            submitted: submitted
        )
        channel_section(
            guild_id: guild_id,
            allowed: page.view.allowed_channels.as_deref(),
            channels: &page.channels,
            locked: page.view.channels_locked,
            added: feedback(submitted, GreetingAction::AddChannel),
            removed: feedback(submitted, GreetingAction::RemoveChannel),
            chosen: typed(submitted, GreetingAction::AddChannel, "channel_id")
                .unwrap_or_default()
        )
        cooldown_section(
            guild_id: guild_id,
            cooldowns: page.view.cooldowns,
            submitted: submitted
        )
        if let Some(outcome) = feedback(submitted, GreetingAction::AddImage) {
            save_feedback(outcome: outcome)
        }
        if let Some(outcome) = feedback(submitted, GreetingAction::RemoveImage) {
            save_feedback(outcome: outcome)
        }
        image_section(
            guild_id: guild_id,
            kind: "morning",
            title: "Good morning images",
            images: &page.view.morning,
            submitted: submitted
        )
        image_section(
            guild_id: guild_id,
            kind: "night",
            title: "Good night images",
            images: &page.view.night,
            submitted: submitted
        )
    }
    .boxed())
}
