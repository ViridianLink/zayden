mod faq;
mod roles;
mod settings;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::header::switch_module;
use super::state::PageState;
use super::{Lists, Page, settings_page};
use crate::guild::dto::SupportSection;
use crate::shell::GuildId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Tickets,
    Suggestions,
    Faq,
    Wiki,
    Roles,
}

impl Pane {
    pub const ALL: [Self; 5] =
        [Self::Tickets, Self::Suggestions, Self::Faq, Self::Wiki, Self::Roles];

    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Tickets => "",
            Self::Suggestions => "/suggestions",
            Self::Faq => "/faq",
            Self::Wiki => "/wiki",
            Self::Roles => "/roles",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Tickets => "Tickets",
            Self::Suggestions => "Suggestions",
            Self::Faq => "FAQ articles",
            Self::Wiki => "Wiki",
            Self::Roles => "Roles and links",
        }
    }
}

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    [settings::routes, roles::routes, faq::routes]
        .into_iter()
        .fold(base.page(switch), |builder, pane| pane(builder))
}

#[page(POST "/guild/{guild_id}/support/module")]
async fn switch(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let page = Page::Support(Pane::Tickets);
    let state = PageState::failed(switch_module(cx, guild_id, page, pairs).await?);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: page, state: &state)
    })
}

#[component]
pub(super) async fn subnav(guild_id: &str, current: Pane) -> Result<impl View> {
    Ok(view! {
        <nav aria-label="Support pages">
            <ul class="page-subnav">
                #[key(pane.label())]
                for pane in Pane::ALL {
                    let here = pane == current;
                    <li>
                        <a
                            href=(Page::Support(pane).href(guild_id))
                            class=(if here {
                                "btn btn-secondary"
                            } else {
                                "btn btn-ghost"
                            })
                            aria-current=(here.then_some("page"))
                        >
                            (pane.label())
                        </a>
                    </li>
                }
            </ul>
        </nav>
    })
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    pane: Pane,
    settings: &SupportSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    Ok(view! {
        match pane {
            Pane::Tickets | Pane::Faq => (view! {
                settings::tickets(
                    guild_id: guild_id,
                    settings: settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            Pane::Suggestions => (view! {
                settings::suggestions(
                    guild_id: guild_id,
                    settings: settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            Pane::Wiki => (view! {
                settings::wiki(
                    guild_id: guild_id,
                    settings: &settings.faq,
                    state: state
                )
            }.boxed(

            )),
            Pane::Roles => (view! {
                roles::pane(
                    guild_id: guild_id,
                    settings: settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
        }
    }
    .boxed())
}
