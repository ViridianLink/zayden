mod faq;
mod roles;
mod settings;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{page, path_param};
use topcoat::runtime::{PrefetchMode, link_attrs};
use topcoat::view::{View, ViewExt, attributes, component, view};

use super::header::switch_module;
use super::state::PageState;
use super::{Lists, Page, settings_page};
use crate::app::guild::guild_id::GuildId;
use crate::components::tabs::{tabs_list, tabs_trigger};
use crate::guild::dto::SupportSection;

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

#[page(POST "./module")]
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
pub(super) async fn subnav(
    cx: &Cx,
    guild_id: &str,
    current: Pane,
) -> Result<impl View> {
    Ok(view! {
        <nav aria-label="Support pages">
            tabs_list(
                attrs: attributes! { class="mb-5" },
                #[key(pane.label())]
                for pane in Pane::ALL {
                    tabs_trigger(
                        active: pane == current,
                        attrs: link_attrs(
                            cx,
                            Page::Support(pane).href(guild_id),
                            PrefetchMode::Intent,
                        ),
                        (pane.label())
                    )
                }
            )
        </nav>
    })
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &SupportSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    Ok(view! {
        match settings {
            SupportSection::Tickets(settings) => (view! {
                settings::tickets(
                    guild_id: guild_id,
                    settings: settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SupportSection::Suggestions(settings) => (view! {
                settings::suggestions(
                    guild_id: guild_id,
                    settings: settings,
                    lists: lists,
                    state: state
                )
            }.boxed(

            )),
            SupportSection::Wiki(settings) => (view! {
                settings::wiki(guild_id: guild_id, settings: settings, state: state)
            }.boxed(

            )),
            SupportSection::Roles(settings) => (view! {
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
