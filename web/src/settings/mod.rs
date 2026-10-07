mod ai;
mod family;
mod faq;
mod general;
mod honeypot;
mod lfg;
mod music;
mod patreon;
mod provider;
mod support;
mod temp_voice;
mod youtube;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::SeeOther;
use topcoat::router::{RouterBuilder, StatusCode, page, path_param};
use topcoat::view::{View, ViewExt, component, view};
use twilight_model::channel::ChannelType;

use crate::components::pickers::{Channel, Role};
use crate::components::settings::save_feedback;
use crate::guild::dto::{GuildDirectory, SectionSettings};
use crate::guild::parse::parse_flag;
use crate::guild::{GuildError, get_guild_directory, get_section_settings};
use crate::nav::{self, ModuleNav};
use crate::shell::GuildId;
use crate::util::server_error_text;

path_param!(section);

const SETTINGS_PATH: &str = "/guild/{guild_id}/settings";

const GENERAL: &str = "general";

const TEXT_KINDS: &[ChannelType] = &[
    ChannelType::GuildText,
    ChannelType::GuildAnnouncement,
    ChannelType::GuildForum,
];

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    let base = base.page(settings_index).page(settings_section);

    [
        general::routes,
        ai::routes,
        family::routes,
        honeypot::routes,
        lfg::routes,
        music::routes,
        temp_voice::routes,
        patreon::routes,
        youtube::routes,
        support::routes,
    ]
    .into_iter()
    .fold(base, |builder, section| section(builder))
}

#[must_use]
pub fn title(slug: &str) -> String {
    let module = nav::section(slug);
    if *module == nav::GENERAL {
        return format!("{} - Zayden Dashboard", module.label);
    }
    format!("{} settings - Zayden Dashboard", module.label)
}

#[must_use]
pub fn route_title(cx: &Cx, pattern: &str) -> Option<String> {
    let rest = pattern.strip_prefix(SETTINGS_PATH)?;
    let slug = match rest {
        "" => "",
        "/{section}" => path_param::<Section>(cx),
        _ => rest.strip_prefix('/')?.split('/').next().unwrap_or_default(),
    };
    Some(title(slug))
}

#[page("/guild/{guild_id}/settings")]
async fn settings_index(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { settings_page(guild_id: guild_id, slug: GENERAL) })
}

#[page("/guild/{guild_id}/settings/{section}")]
async fn settings_section(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let slug: &str = path_param::<Section>(cx);

    Ok(view! { settings_page(guild_id: guild_id, slug: slug) })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Submission {
    form: &'static str,
    values: Vec<(String, String)>,
    result: std::result::Result<(), String>,
    reloads: bool,
}

impl Submission {
    pub(super) fn new(
        form: &'static str,
        values: Vec<(String, String)>,
        result: std::result::Result<(), GuildError>,
    ) -> std::result::Result<Self, SeeOther> {
        Ok(Self { form, values, result: message(result)?, reloads: false })
    }

    pub(super) fn reloading(
        form: &'static str,
        result: std::result::Result<(), GuildError>,
    ) -> std::result::Result<Self, SeeOther> {
        Ok(Self {
            form,
            values: Vec::new(),
            result: message(result)?,
            reloads: true,
        })
    }

    pub(super) const fn succeeded(form: &'static str) -> Self {
        Self { form, values: Vec::new(), result: Ok(()), reloads: true }
    }

    pub(super) const fn status(&self) -> StatusCode {
        if self.result.is_ok() {
            StatusCode::OK
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        }
    }

    pub(super) fn of<'a>(this: Option<&'a Self>, form: &str) -> Option<&'a Self> {
        this.filter(|submission| submission.form == form)
    }

    pub(super) fn outcome(&self) -> std::result::Result<(), &str> {
        match &self.result {
            Ok(()) => Ok(()),
            Err(message) => Err(message),
        }
    }

    fn failed_save(&self) -> Option<&str> {
        if self.reloads { None } else { self.outcome().err() }
    }

    fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

fn message(
    result: std::result::Result<(), GuildError>,
) -> std::result::Result<std::result::Result<(), String>, SeeOther> {
    match result {
        Ok(()) => Ok(Ok(())),
        Err(error) => Ok(Err(error.redirect_unauthenticated()?.to_string())),
    }
}

pub(super) fn shown<'a>(
    submission: Option<&'a Submission>,
    name: &str,
    stored: Option<&'a str>,
) -> &'a str {
    submission
        .and_then(|submission| submission.value(name))
        .or(stored)
        .unwrap_or_default()
}

pub(super) fn flag(
    submission: Option<&Submission>,
    name: &str,
    stored: bool,
) -> bool {
    submission
        .and_then(|submission| submission.value(name))
        .map_or(stored, parse_flag)
}

pub(super) fn ensure_path_guild(
    form_guild: &str,
    path_guild: &str,
) -> std::result::Result<(), GuildError> {
    if form_guild == path_guild {
        Ok(())
    } else {
        Err(GuildError::InvalidField("guild"))
    }
}

pub(super) fn action(guild_id: &str, slug: &str) -> String {
    format!("/guild/{guild_id}/settings/{slug}")
}

pub(super) struct Lists {
    channels: std::result::Result<Vec<Channel>, String>,
    roles: std::result::Result<Vec<Role>, String>,
}

impl Lists {
    pub(super) fn channels(&self) -> std::result::Result<&[Channel], &str> {
        self.channels.as_deref().map_err(String::as_str)
    }

    pub(super) fn roles(&self) -> std::result::Result<&[Role], &str> {
        self.roles.as_deref().map_err(String::as_str)
    }
}

impl From<GuildDirectory> for Lists {
    fn from(directory: GuildDirectory) -> Self {
        Self {
            channels: directory
                .channels
                .map(|channels| channels.into_iter().map(Channel::from).collect()),
            roles: directory
                .roles
                .map(|roles| roles.into_iter().map(Role::from).collect()),
        }
    }
}

#[component]
pub(super) async fn settings_page(
    guild_id: &str,
    slug: &str,
    #[default] submission: Option<&Submission>,
) -> Result<impl View> {
    let module = nav::section(slug);

    Ok(view! {
        <div class="page">
            page_header(module: module)
            section_panel(
                guild_id: guild_id,
                slug: module.slug().unwrap_or(GENERAL),
                submission: submission
            )
        </div>
    })
}

#[component]
async fn page_header(module: &ModuleNav) -> Result<impl View> {
    Ok(view! {
        <div class="page-header">
            <div>
                <h1>(module.label)</h1>
                <p class="page-lead">(module.lead())</p>
            </div>
        </div>
    })
}

#[component]
async fn section_panel(
    cx: &Cx,
    guild_id: &str,
    slug: &str,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let (directory, settings) = tokio::join!(
        get_guild_directory(cx, guild_id),
        get_section_settings(cx, guild_id, slug),
    );
    let failed_save = submission.and_then(Submission::failed_save);
    let loaded = match (directory, settings) {
        (Err(error), _) | (_, Err(error)) => Err(server_error_text(error)),
        (Ok(directory), Ok(settings)) => Ok((Lists::from(directory), settings)),
    };

    Ok(view! {
        match loaded {
            Err(error) => {
                if let Some(detail) = failed_save {
                    save_feedback(outcome: Err(detail))
                }
                <p class="error">
                    "Failed to load settings: "
                    (error)
                </p>
            }
            Ok((lists, settings)) => section_tab(
                guild_id: guild_id,
                lists: &lists,
                settings: settings,
                submission: submission
            ),
        }
    })
}

#[component]
async fn section_tab(
    guild_id: &str,
    lists: &Lists,
    settings: SectionSettings,
    submission: Option<&Submission>,
) -> Result<impl View> {
    Ok(view! {
        match settings {
            SectionSettings::General(settings) => (view! {
                general::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    submission: submission
                )
            }.boxed(

            )),
            SectionSettings::Ai(settings) => (view! {
                ai::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    submission: submission
                )
            }.boxed(

            )),
            SectionSettings::Family(settings) => (view! {
                family::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    submission: submission
                )
            }.boxed(

            )),
            SectionSettings::Honeypot(settings) => (view! {
                honeypot::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    submission: submission
                )
            }.boxed(

            )),
            SectionSettings::Lfg(settings) => (view! {
                lfg::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    submission: submission
                )
            }.boxed(

            )),
            SectionSettings::Music(settings) => (view! {
                music::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    submission: submission
                )
            }.boxed(

            )),
            SectionSettings::TempVoice(settings) => (view! {
                temp_voice::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    submission: submission
                )
            }.boxed(

            )),
            SectionSettings::Patreon(status) => (view! {
                patreon::tab(
                    guild_id: guild_id,
                    status: &status,
                    lists: lists,
                    submission: submission
                )
            }.boxed(

            )),
            SectionSettings::Youtube(status) => (view! {
                youtube::tab(
                    guild_id: guild_id,
                    status: &status,
                    lists: lists,
                    submission: submission
                )
            }.boxed(

            )),
            SectionSettings::Support(settings) => (view! {
                support::tab(
                    guild_id: guild_id,
                    settings: settings.as_ref(),
                    lists: lists,
                    submission: submission
                )
            }.boxed(

            )),
        }
    })
}
