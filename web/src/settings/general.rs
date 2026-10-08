use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::fields::{channels, form_summary, roles, select_row};
use super::state::{Done, PageState, settle};
use super::{
    Lists,
    Page,
    ROLE_ORDER_NOTE,
    TEXT_KINDS,
    ensure_path_guild,
    settings_page,
};
use crate::components::save_bar::save_bar;
use crate::guild::GuildError;
use crate::guild::dto::GeneralSection;
use crate::guild::settings::{ServerSettingsForm, save_server_settings};
use crate::shell::GuildId;

const PAGE: Page = Page::Server;
const FORM: &str = "server-settings";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(server_settings).page(save)
}

#[page("/guild/{guild_id}/settings")]
async fn server_settings(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/settings")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = save_server(cx, guild_id, pairs).await;
    let failure = settle(cx, FORM, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(FORM),
        message: "Server settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

async fn save_server(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = ServerSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_server_settings(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &GeneralSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let sent = state.sent(FORM);
    let action = PAGE.href(guild_id);
    let row = |name: &str, label: &str, stored: Option<&str>| {
        (name.to_owned(), label.to_owned(), sent.value(name, stored).to_owned())
    };
    let channel_rows = [
        row(
            "rules_channel_id",
            "Rules channel",
            settings.rules_channel_id.as_deref(),
        ),
        row(
            "general_channel_id",
            "General channel",
            settings.general_channel_id.as_deref(),
        ),
        row(
            "spoiler_channel_id",
            "Spoiler channel",
            settings.spoiler_channel_id.as_deref(),
        ),
    ];
    let role_rows = [
        row("artist_role_id", "Artist role", settings.artist_role_id.as_deref()),
        row("sleep_role_id", "Sleep role", settings.sleep_role_id.as_deref()),
        row(
            "verified_role_id",
            "Verified role",
            settings.verified_role_id.as_deref(),
        ),
    ];

    Ok(view! {
        <form
            id=(FORM)
            method="post"
            action=(action.as_str())
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = sent.summary() {
                form_summary(form: FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Channels"</legend>
                <p class="page-lead">
                    "Where Zayden points members for rules, chat and spoilers."
                </p>
                #[key(name.as_str())]
                for (name, label, selected) in &channel_rows {
                    select_row(
                        form: FORM,
                        name: name,
                        label: label,
                        selected: selected,
                        options: channels(lists, TEXT_KINDS),
                        error: sent.error(name)
                    )
                }
            </fieldset>
            <fieldset class="settings-section">
                <legend>"Roles"</legend>
                <p class="page-lead">
                    "Roles Zayden gives out for art, sleep and verification."
                </p>
                <p class="field-hint">(ROLE_ORDER_NOTE)</p>
                #[key(name.as_str())]
                for (name, label, selected) in &role_rows {
                    select_row(
                        form: FORM,
                        name: name,
                        label: label,
                        selected: selected,
                        options: roles(lists),
                        error: sent.error(name)
                    )
                }
            </fieldset>
            save_bar(notice: state.notice_for(FORM))
        </form>
    }
    .boxed())
}
