mod channels;
mod cooldowns;
mod images;
mod messages;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::not_found;
use topcoat::router::{page, path_param};
use topcoat::view::{View, ViewExt, component, view};

pub use self::channels::channel_section;
use self::cooldowns::cooldown_section;
use self::images::image_section;
use self::messages::messages_section;
use super::action::{FormAction, form_action, not_saved, page_href};
use super::fields::{form_summary, load_error, load_problem};
use super::header::{MODULE_FORM, page_header, switch_module};
use super::state::{Done, PageState, rerender, settle};
use crate::components::flash::flash;
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

path_param!(action);

const PAGE: &str = "greetings";
const TITLE: &str = "Greetings";
const MODULE_ID: &str = "greetings";

const MESSAGES: &str = "greeting-messages";
const COOLDOWNS: &str = "greeting-cooldowns";
const CHANNELS: &str = "good-channels";
const IMAGES: &str = "greeting-images";

const ADD_CHANNEL: &str = "add-good-channel";
const REMOVE_CHANNEL: &str = "remove-good-channel";
const REMOVE_IMAGE: &str = "remove-greeting-image";

#[derive(Clone, Copy, PartialEq, Eq)]
enum GreetingAction {
    SaveMessages,
    SaveCooldowns,
    AddChannel,
    RemoveChannel,
    AddImage,
    RemoveImage,
    Module,
}

impl FormAction for GreetingAction {
    const ALL: &'static [Self] = &[
        Self::SaveMessages,
        Self::SaveCooldowns,
        Self::AddChannel,
        Self::RemoveChannel,
        Self::AddImage,
        Self::RemoveImage,
        Self::Module,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::SaveMessages => "save-messages",
            Self::SaveCooldowns => "save-cooldowns",
            Self::AddChannel => "add-channel",
            Self::RemoveChannel => "remove-channel",
            Self::AddImage => "add-image",
            Self::RemoveImage => "remove-image",
            Self::Module => "module",
        }
    }
}

fn add_image_form(kind: &str) -> &'static str {
    if kind == "night" { "add-night-image" } else { "add-morning-image" }
}

impl GreetingAction {
    fn form(self, values: &[(String, String)]) -> &'static str {
        match self {
            Self::SaveMessages => MESSAGES,
            Self::SaveCooldowns => COOLDOWNS,
            Self::AddChannel => ADD_CHANNEL,
            Self::RemoveChannel => REMOVE_CHANNEL,
            Self::AddImage => add_image_form(
                values
                    .iter()
                    .find(|(name, _)| name == "kind")
                    .map_or("", |(_, kind)| kind.as_str()),
            ),
            Self::RemoveImage => REMOVE_IMAGE,
            Self::Module => MODULE_FORM,
        }
    }

    const fn section(self) -> Option<&'static str> {
        match self {
            Self::SaveMessages => Some(MESSAGES),
            Self::SaveCooldowns => Some(COOLDOWNS),
            Self::AddChannel | Self::RemoveChannel => Some(CHANNELS),
            Self::AddImage | Self::RemoveImage => Some(IMAGES),
            Self::Module => None,
        }
    }

    const fn done(self) -> &'static str {
        match self {
            Self::SaveMessages => "Greeting messages saved.",
            Self::SaveCooldowns => "Cooldowns saved.",
            Self::AddChannel => "Channel added to the /good list.",
            Self::RemoveChannel => "Channel removed from the /good list.",
            Self::AddImage => "Image added.",
            Self::RemoveImage => "Image removed.",
            Self::Module => "",
        }
    }
}

pub(in crate::engagement::pages) fn action_names() -> Vec<&'static str> {
    GreetingAction::ALL.iter().map(|action| action.name()).collect()
}

#[page("/guild/{guild_id}/greetings")]
pub(super) async fn show(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! {
        (state.status())
        greetings_page(guild_id: guild_id, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/greetings")]
pub(super) async fn legacy_submit(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    not_saved(cx, guild_id, PAGE)?;
    Ok(view! { "" })
}

#[page(POST "/guild/{guild_id}/greetings/{action}")]
pub(super) async fn submit(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let Some(action) = GreetingAction::find(path_param::<Action>(cx)) else {
        return Err(not_found().into());
    };
    let page = page_href(guild_id, PAGE);

    let failure = if action == GreetingAction::Module {
        switch_module(cx, guild_id, MODULE_ID, TITLE, page.clone(), pairs).await?
    } else {
        let form = action.form(&pairs);
        let values = pairs.clone();
        let result = save(cx, action, guild_id, pairs).await;
        settle(cx, form, values, result, &Done {
            page: page.clone(),
            section: action.section(),
            message: action.done(),
        })?
    };
    Err::<(), _>(rerender(cx, &page, failure))?;
    Ok(view! { "" })
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
        GreetingAction::Module => Err(EngagementError::Invalid("form")),
    }
}

#[component]
async fn greetings_page(
    cx: &Cx,
    guild_id: &str,
    state: &PageState,
) -> Result<impl View> {
    let data = match load_greetings_page(cx, guild_id).await {
        Ok(page) => Ok(page),
        Err(error) => Err(error.redirect_unauthenticated()?),
    };
    let switch = form_action(guild_id, PAGE, GreetingAction::Module);

    Ok(view! {
        <div class="page">
            page_header(
                guild_id: guild_id,
                title: TITLE,
                module_id: Some(MODULE_ID),
                switch: Some(&switch),
                failure: state.sent(MODULE_FORM).summary(),
                "What Zayden posts for "
                <code>"/good morning"</code>
                " and "
                <code>"/good night"</code>
                ". Each subcommand replies with one image picked at random from its list, plus the message below if you set one."
            )
            flash(notice: state.top_notice())
            match data {
                Err(error) => {
                    let (message, actions) =
                        load_problem(guild_id, &page_href(guild_id, PAGE), &error);
                    if let Some(failure) = state.any_failure() {
                        form_summary(form: "page", message: failure)
                    }
                    load_error(
                        title: "Couldn't load the greetings",
                        message: &message,
                        actions: &actions
                    )
                }
                Ok(page) => greeting_sections(guild_id: guild_id, page: &page, state: state),
            }
        </div>
    }
    .boxed())
}

#[component]
async fn greeting_sections(
    guild_id: &str,
    page: &GreetingsPage,
    state: &PageState,
) -> Result<impl View> {
    let added = state.sent(ADD_CHANNEL);
    let removed = state.sent(REMOVE_CHANNEL);

    Ok(view! {
        messages_section(
            guild_id: guild_id,
            morning: page.view.morning_message.as_str(),
            night: page.view.night_message.as_str(),
            state: state
        )
        channel_section(
            guild_id: guild_id,
            allowed: page.view.allowed_channels.as_deref(),
            channels: &page.channels,
            locked: page.view.channels_locked,
            notice: state.notice_for(CHANNELS),
            add_error: added.summary(),
            add_field_error: added.error("channel_id"),
            remove_error: removed.summary(),
            chosen: added.value("channel_id", "")
        )
        cooldown_section(guild_id: guild_id, cooldowns: page.view.cooldowns, state: state)
        image_section(
            guild_id: guild_id,
            morning: &page.view.morning,
            night: &page.view.night,
            state: state
        )
    }
    .boxed())
}
