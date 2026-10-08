use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::see_other;
use topcoat::router::{StatusCode, page, path_param};
use topcoat::view::{View, ViewExt, component, view};
use zayden_app::modules;

use super::card::module_card;
use super::guild_layout::GuildId;
use crate::auth::AuthError;
use crate::components::error_panel::{ErrorAction, error_panel};
use crate::components::flash::flash;
use crate::components::module_row::module_group;
use crate::flash::{Flash, FlashKind, set_in as set_flash, take as take_flash};
use crate::guild::dto::ModuleView;
use crate::guild::modules::{
    ModuleToggleForm,
    list_guild_modules,
    set_module_enabled,
};
use crate::guild::{GuildError, list_manageable_guilds};
use crate::nav::{self, path_segment};
use crate::util::server_error_text;

const LEAD: &str = "Turn modules on or off for this server. A module that's off \
                    has its commands removed from the server.";

struct RackGroup {
    slug: &'static str,
    title: &'static str,
    modules: &'static [&'static str],
}

const RACK: &[RackGroup] = &[
    RackGroup {
        slug: "community",
        title: "Community",
        modules: &["greetings", "family", "gambling", "misc"],
    },
    RackGroup {
        slug: "support",
        title: "Support & safety",
        modules: &["ticket", "honeypot", "moderation"],
    },
    RackGroup {
        slug: "voice",
        title: "Voice & games",
        modules: &["music", "jellyfin", "palworld", "marathon", "hosting"],
    },
    RackGroup {
        slug: "integrations",
        title: "Integrations",
        modules: &["ai", "patreon", "youtube"],
    },
];

const OTHER: RackGroup =
    RackGroup { slug: "other", title: "Other modules", modules: &[] };

pub(super) fn plain(message: &str) -> &str {
    let prefix = server_error_text("");
    message.strip_prefix(prefix.as_str()).unwrap_or(message)
}

fn section_id(group: &RackGroup) -> String {
    format!("modules-{}", group.slug)
}

fn group_of(module_id: &str) -> &'static RackGroup {
    RACK.iter().find(|group| group.modules.contains(&module_id)).unwrap_or(&OTHER)
}

fn overview_href(guild_id: &str) -> String {
    format!("/guild/{}", path_segment(guild_id))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToggleFailure {
    module_id: Option<String>,
    message: String,
}

#[page("/guild/{guild_id}")]
pub(super) async fn guild_overview(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let notice = take_flash(cx);
    let loaded = load(cx, guild_id).await?;

    Ok(view! {
        overview(guild_id: guild_id, loaded: loaded, notice: notice.as_ref())
    })
}

#[page(POST "/guild/{guild_id}")]
pub(super) async fn toggle_module(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let module_id = pairs
        .iter()
        .find(|(name, _)| name == "module_id")
        .map(|(_, value)| value.clone());

    let error = match toggle(cx, guild_id, pairs).await {
        Ok((module_id, enabled)) => {
            let label = modules::find(&module_id)
                .map_or(module_id.as_str(), |module| module.label);
            let message =
                format!("{label} turned {}.", if enabled { "on" } else { "off" });
            let section = section_id(group_of(&module_id));
            set_flash(cx, FlashKind::Success, &message, &section)?;
            let location = format!("{}#{section}", overview_href(guild_id));
            return Err(see_other(location).into());
        },
        Err(error) => error.redirect_unauthenticated()?,
    };
    let failure = ToggleFailure {
        module_id,
        message: format!("Not changed: {}", plain(&error.to_string())),
    };
    let loaded = load(cx, guild_id).await?;

    Ok(view! {
        (StatusCode::UNPROCESSABLE_ENTITY)
        overview(guild_id: guild_id, loaded: loaded, failure: Some(failure))
    })
}

async fn toggle(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(String, bool), GuildError> {
    let form = ModuleToggleForm::from_pairs(pairs)?;
    if form.guild != guild_id {
        return Err(GuildError::InvalidField("guild"));
    }
    let enabled = form.enabled()?;

    set_module_enabled(cx, guild_id, &form.module_id, enabled).await?;
    Ok((form.module_id, enabled))
}

enum Loaded {
    Modules(Vec<ModuleView>),
    Problem { title: &'static str, message: String, actions: Vec<ErrorAction> },
}

async fn load(cx: &Cx, guild_id: &str) -> Result<Loaded> {
    let error = match list_guild_modules(cx, guild_id).await {
        Ok(modules) => return Ok(Loaded::Modules(modules)),
        Err(error) => error.redirect_unauthenticated()?,
    };
    let back = ErrorAction::new("Back to servers", "/guilds");

    Ok(if error == GuildError::Auth(AuthError::BotNotInGuild) {
        let name = server_name(cx, guild_id).await;
        let invite = format!("/invite?guild={}", path_segment(guild_id));
        let (message, action) = name.as_deref().map_or_else(
            || {
                (
                    "Add Zayden to this server to turn its modules on or off here."
                        .to_owned(),
                    "Add Zayden to this server".to_owned(),
                )
            },
            |name| {
                (
                    format!(
                        "Add Zayden to {name} to turn its modules on or off here."
                    ),
                    format!("Add Zayden to {name}"),
                )
            },
        );
        Loaded::Problem {
            title: "Zayden isn't in this server yet",
            message,
            actions: vec![ErrorAction::new(&action, &invite).external(), back],
        }
    } else if error == GuildError::Auth(AuthError::InvalidGuildId) {
        Loaded::Problem {
            title: "Server not found",
            message: "That address doesn't name a Discord server.".to_owned(),
            actions: vec![back],
        }
    } else if error.is_denied() {
        Loaded::Problem {
            title: "You can't manage this server",
            message: "You need Manage Server in this server to change its modules."
                .to_owned(),
            actions: vec![back],
        }
    } else {
        Loaded::Problem {
            title: "Couldn't load this server's modules",
            message: plain(&error.to_string()).to_owned(),
            actions: vec![
                ErrorAction::new("Try again", &overview_href(guild_id)),
                back,
            ],
        }
    })
}

async fn server_name(cx: &Cx, guild_id: &str) -> Option<String> {
    list_manageable_guilds(cx)
        .await
        .ok()?
        .into_iter()
        .find(|guild| guild.id == guild_id)
        .map(|guild| guild.name)
}

#[component]
async fn overview(
    guild_id: &str,
    loaded: Loaded,
    #[default] notice: Option<&Flash>,
    #[default] failure: Option<ToggleFailure>,
) -> Result<impl View> {
    Ok(view! {
        <div class="page">
            match loaded {
                Loaded::Problem { title, message, actions } => error_panel(
                    title: title,
                    message: &message,
                    actions: &actions
                ),
                Loaded::Modules(modules) => {
                    <div class="page-header">
                        <div>
                            <h1>"Overview"</h1>
                            <p class="page-lead">(LEAD)</p>
                        </div>
                        <a href=(nav::GENERAL.href(guild_id)) class="btn btn-secondary">
                            "Server settings"
                        </a>
                    </div>
                    flash(notice: notice.filter(|notice| notice.section.is_none()))
                    rack(
                        guild_id: guild_id,
                        modules: &modules,
                        notice: notice,
                        failure: failure.as_ref()
                    )
                }
            }
        </div>
    }
    .boxed())
}

#[component]
async fn rack(
    guild_id: &str,
    modules: &[ModuleView],
    notice: Option<&Flash>,
    failure: Option<&ToggleFailure>,
) -> Result<impl View> {
    let failed_module = failure.and_then(|failure| failure.module_id.as_deref());
    let unattached = failure
        .filter(|_| {
            !modules.iter().any(|module| Some(module.id.as_str()) == failed_module)
        })
        .map(|failure| failure.message.as_str());
    let others: Vec<&ModuleView> = modules
        .iter()
        .filter(|module| {
            !RACK.iter().any(|group| group.modules.contains(&module.id.as_str()))
        })
        .collect();
    let mut groups: Vec<(&RackGroup, Vec<&ModuleView>)> = RACK
        .iter()
        .map(|group| {
            let rows = group
                .modules
                .iter()
                .filter_map(|id| modules.iter().find(|module| module.id == *id))
                .collect();
            (group, rows)
        })
        .collect();
    groups.push((&OTHER, others));
    groups.retain(|(_, rows)| !rows.is_empty());

    Ok(view! {
        if let Some(message) = unattached {
            <p class="error" role="alert">(message)</p>
        }
        #[key(group.slug)]
        for (group, rows) in &groups {
            let id = section_id(group);
            let group_notice = notice.filter(|notice| notice.is_for(&id));
            <div id=(id.as_str())>
                if group_notice.is_some() {
                    flash(notice: group_notice)
                }
                module_group(
                    title: group.title,
                    #[key(module.id.as_str())]
                    for module in rows {
                        let error = failure
                            .filter(|_| failed_module == Some(module.id.as_str()))
                            .map(|failure| failure.message.as_str());
                        module_card(module: module, guild_id: guild_id, error: error)
                    }
                )
            </div>
        }
    }
    .boxed())
}
