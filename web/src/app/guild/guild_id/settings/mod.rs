use topcoat::router::segment;
mod ai;
mod announce;
mod deadlock;
mod family;
mod faq;
mod fields;
mod general;
mod header;
mod honeypot;
mod legacy;
mod lfg;
mod music;
mod patreon;
mod provider;
mod state;
mod support;
mod temp_voice;
mod youtube;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{Child, View, ViewExt, attributes, component, view};
use twilight_model::channel::ChannelType;

use self::header::feature_header;
pub use self::legacy::NOT_SAVED;
use self::state::{PageState, plain};
pub use self::support::Pane;
use crate::auth::AuthError;
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::components::error_panel::ErrorAction;
use crate::components::field_row::form_summary;
use crate::components::flash::flash;
use crate::components::pickers::{Channel, Role};
use crate::document::PAGE_TITLES;
use crate::guild::dto::{GuildDirectory, SectionSettings};
use crate::guild::{GuildError, get_guild_directory, get_section_settings};
use crate::nav::{self, ModuleNav};

segment!(kind = Group);

const TEXT_KINDS: &[ChannelType] = &[
    ChannelType::GuildText,
    ChannelType::GuildAnnouncement,
    ChannelType::GuildForum,
];

const GUILD_PREFIX: &str = "/guild/{guild_id}/";

const TITLE_SUFFIX: &str = " - Zayden Dashboard";

const ROLE_ORDER_NOTE: &str = "Zayden's role must be above the roles it \
                               assigns. In Discord, drag it above them under \
                               Server Settings > Roles.";

#[must_use]
pub fn title(entry: &ModuleNav) -> String {
    format!("{}{TITLE_SUFFIX}", entry.label)
}

#[must_use]
pub fn route_title(_cx: &Cx, pattern: &str) -> Option<String> {
    let feature = pattern.strip_prefix(GUILD_PREFIX)?.split('/').next()?;
    nav::settings_entry(feature)?;

    let mut candidate = pattern;
    loop {
        if let Some((_, title)) =
            PAGE_TITLES.iter().find(|(route, _)| *route == candidate)
        {
            return Some((*title).to_owned());
        }
        candidate = candidate.rsplit_once('/')?.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Server,
    Ai,
    Family,
    Honeypot,
    Lfg,
    Music,
    TempVoice,
    Patreon,
    Youtube,
    Deadlock,
    Support(Pane),
}

impl Page {
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Server => "general",
            Self::Ai => "ai",
            Self::Family => "family",
            Self::Honeypot => "honeypot",
            Self::Lfg => "lfg",
            Self::Music => "music",
            Self::TempVoice => "temp-voice",
            Self::Patreon => "patreon",
            Self::Youtube => "youtube",
            Self::Deadlock => "deadlock",
            Self::Support(_) => "support",
        }
    }

    #[must_use]
    pub fn entry(self) -> &'static ModuleNav {
        nav::section(self.slug())
    }

    #[must_use]
    pub fn href(self, guild_id: &str) -> String {
        let base = self.entry().href(guild_id);
        match self.pane() {
            Some(pane) => format!("{base}{}", pane.suffix()),
            None => base,
        }
    }

    #[must_use]
    pub const fn section(self) -> &'static str {
        match self {
            Self::Support(Pane::Suggestions) => "support-suggestions",
            Self::Support(Pane::Wiki) => "support-wiki",
            Self::Support(Pane::Roles) => "support-roles",
            Self::Support(Pane::Tickets | Pane::Faq) => "support",
            Self::Server
            | Self::Ai
            | Self::Family
            | Self::Honeypot
            | Self::Lfg
            | Self::Music
            | Self::TempVoice
            | Self::Patreon
            | Self::Youtube
            | Self::Deadlock => self.slug(),
        }
    }

    #[must_use]
    pub const fn pane(self) -> Option<Pane> {
        if let Self::Support(pane) = self { Some(pane) } else { None }
    }
}

fn ensure_path_guild(
    form_guild: &str,
    path_guild: &str,
) -> std::result::Result<(), GuildError> {
    if form_guild == path_guild {
        Ok(())
    } else {
        Err(GuildError::InvalidField("guild"))
    }
}

struct Lists {
    channels: std::result::Result<Vec<Channel>, String>,
    roles: std::result::Result<Vec<Role>, String>,
}

impl Lists {
    fn channels(&self) -> std::result::Result<&[Channel], &str> {
        self.channels.as_deref().map_err(String::as_str)
    }

    fn roles(&self) -> std::result::Result<&[Role], &str> {
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
async fn frame(
    guild_id: &str,
    page: Page,
    state: &PageState,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class="page">
            feature_header(guild_id: guild_id, page: page, state: state)
            if let Page::Support(pane) = page {
                support::subnav(guild_id: guild_id, current: pane)
                <h2
                    class="m-0 mb-4 text-[17px] font-semibold tracking-[-0.01em] text-text"
                >
                    (pane.label())
                </h2>
            }
            flash(notice: state.top_notice())
            (child)
        </div>
    }
    .boxed())
}

#[component]
async fn settings_page(
    guild_id: &str,
    page: Page,
    state: &PageState,
) -> Result<impl View> {
    Ok(view! {
        frame(
            guild_id: guild_id,
            page: page,
            state: state,
            section_panel(guild_id: guild_id, page: page, state: state)
        )
    }
    .boxed())
}

fn load_problem(
    guild_id: &str,
    retry: &str,
    error: &GuildError,
) -> (String, Vec<ErrorAction>) {
    let back = ErrorAction::new("Back to servers", "/guilds");
    if matches!(error, GuildError::Auth(AuthError::BotNotInGuild)) {
        let invite = format!("/invite?guild={}", nav::path_segment(guild_id));
        return ("Zayden isn't in this server yet.".to_owned(), vec![
            ErrorAction::new("Add Zayden to this server", &invite).external(),
            back,
        ]);
    }
    let message = if error.is_denied() {
        "You need Manage Server in this server to change its settings.".to_owned()
    } else if matches!(error, GuildError::Auth(AuthError::InvalidGuildId)) {
        "That address doesn't name a Discord server.".to_owned()
    } else {
        format!("Something went wrong: {}", plain(&error.to_string()))
    };
    (message, vec![ErrorAction::new("Try again", retry), back])
}

#[component]
async fn load_error(
    cx: &Cx,
    guild_id: &str,
    page: Page,
    error: &GuildError,
    #[default] failure: Option<&str>,
) -> Result<impl View> {
    let (message, actions) = load_problem(guild_id, &page.href(guild_id), error);

    Ok(view! {
        if let Some(failure) = failure {
            form_summary(form: "page", message: failure)
        }
        <section
            class="flex flex-col gap-3 max-w-(--form-max) p-6 border border-solid border-danger rounded-panel bg-surface-1"
            role="alert"
            aria-labelledby="load-error-title"
        >
            <h2 class="error-title" id="load-error-title">
                "Couldn't load these settings"
            </h2>
            <p class="m-0 text-text-2">(message)</p>
            <div class="flex flex-wrap gap-2">
                #[key(index)]
                for (index, action) in actions.iter().enumerate() {
                    let variant = if index == 0 {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Secondary
                    };
                    let link = if action.external {
                        attributes! { href=(action.href.as_str()) rel="external" }
                    } else {
                        link_attrs(cx, action.href.as_str(), prefetch_mode(cx))
                    };
                    <a class=(button_variants(variant, ButtonSize::Md)) (link)>
                        (action.label.as_str())
                    </a>
                }
            </div>
        </section>
    })
}

#[component]
async fn section_panel(
    cx: &Cx,
    guild_id: &str,
    page: Page,
    state: &PageState,
) -> Result<impl View> {
    let slug = page.section();
    let (directory, settings) = tokio::join!(
        get_guild_directory(cx, guild_id),
        get_section_settings(cx, guild_id, slug),
    );
    let loaded = match (directory, settings) {
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(directory), Ok(settings)) => Ok((Lists::from(directory), settings)),
    };

    Ok(view! {
        match loaded {
            Err(error) => load_error(
                guild_id: guild_id,
                page: page,
                error: &error,
                failure: state.any_failure()
            ),
            Ok((lists, settings)) => section_tab(
                guild_id: guild_id,
                lists: &lists,
                settings: settings,
                state: state
            ),
        }
    }
    .boxed())
}

#[component]
async fn section_tab(
    guild_id: &str,
    lists: &Lists,
    settings: SectionSettings,
    state: &PageState,
) -> Result<impl View> {
    Ok(view! {
        match settings {
            SectionSettings::General(settings) => (view! {
                general::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SectionSettings::Ai(settings) => (view! {
                ai::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SectionSettings::Deadlock(settings) => (view! {
                deadlock::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SectionSettings::Family(settings) => (view! {
                family::tab(guild_id: guild_id, settings: &settings, state: state)
            }.boxed(

            )),
            SectionSettings::Honeypot(settings) => (view! {
                honeypot::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SectionSettings::Lfg(settings) => (view! {
                lfg::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SectionSettings::Music(settings) => (view! {
                music::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SectionSettings::TempVoice(settings) => (view! {
                temp_voice::tab(
                    guild_id: guild_id,
                    settings: &settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SectionSettings::Patreon(status) => (view! {
                patreon::tab(
                    guild_id: guild_id,
                    status: &status,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SectionSettings::Youtube(status) => (view! {
                youtube::tab(
                    guild_id: guild_id,
                    status: &status,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SectionSettings::Support(settings) => (view! {
                support::tab(
                    guild_id: guild_id,
                    settings: settings.as_ref(),
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
        }
    }
    .boxed())
}
