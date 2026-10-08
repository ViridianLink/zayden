#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::view::{Child, View, ViewExt, component, view};

use crate::auth::{SessionUser, check_session, current_session_user};
use crate::components::brand::brand_mark;
use crate::components::guild_grid::GuildCard;
use crate::components::icons::{Icon, icon};
use crate::components::nav_links::NavAccess;
use crate::components::nav_rail::nav_rail;
use crate::components::nav_sheet::{SHEET_ID, nav_sheet};
use crate::components::progress_bar::progress_bar;
use crate::components::server_plate::server_plate;
use crate::guild::dto::{GuildInfo, Tier};
use crate::guild::get_active_guild;
use crate::guild::tier::get_user_tier;

const ACCOUNT_MENU_ID: &str = "account-menu";
const ACCOUNT_FALLBACK: &str = "Account";

struct Viewer {
    session: Option<bool>,
    tier: Option<Tier>,
    access: NavAccess,
    current: Option<GuildCard>,
    user: Option<SessionUser>,
}

impl Viewer {
    async fn load(cx: &Cx, guild_id: Option<&str>, guilds: &[GuildInfo]) -> Self {
        let session = check_session(cx).await.ok();
        if session != Some(true) {
            return Self {
                session,
                tier: None,
                access: NavAccess::default(),
                current: None,
                user: None,
            };
        }
        let (tier, access, current, user) = tokio::join!(
            get_user_tier(cx),
            NavAccess::load(cx, guild_id),
            current_guild(cx, guild_id, guilds),
            current_session_user(cx),
        );

        Self {
            session,
            tier: tier.ok().and_then(|info| info.tier),
            access,
            current,
            user: user.ok().flatten(),
        }
    }
}

async fn current_guild(
    cx: &Cx,
    guild_id: Option<&str>,
    guilds: &[GuildInfo],
) -> Option<GuildCard> {
    let guild_id = guild_id?;
    get_active_guild(cx, guild_id).await.map_or_else(
        |_| guilds.iter().find(|guild| guild.id == guild_id).map(GuildCard::from),
        |guild| Some(GuildCard::from(&guild)),
    )
}

#[component]
pub(super) async fn frame(
    cx: &Cx,
    #[default] guild_id: Option<&str>,
    #[default] guilds: &[GuildInfo],
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let viewer = Box::pin(Viewer::load(cx, guild_id, guilds)).await;

    Ok(view! {
        <div class="app">
            <header class="app-header">
                topbar(viewer: &viewer, show_plate: guild_id.is_some(), guilds: guilds)
                progress_bar()
                nav_rail(access: viewer.access, guild_id: guild_id)
                nav_sheet(access: viewer.access, guild_id: guild_id)
            </header>
            <main id="main" class="app-main" tabindex="-1">(child)</main>
        </div>
    }
    .boxed())
}

#[component]
pub async fn app_shell(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { frame((child)) })
}

#[component]
async fn topbar(
    viewer: &Viewer,
    show_plate: bool,
    #[default] guilds: &[GuildInfo],
) -> Result<impl View> {
    let home = if viewer.session == Some(true) { "/guilds" } else { "/" };

    Ok(view! {
        <div class="topbar">
            <button
                type="button"
                class="topbar-button menu-button"
                popovertarget=(SHEET_ID)
                aria-controls=(SHEET_ID)
                aria-expanded="false"
                aria-label="Menu"
            >
                icon(name: Icon::Menu)
            </button>
            <a href=(home) class="brand">
                brand_mark()
                <span class="brand-name">"Zayden"</span>
            </a>
            if show_plate {
                server_plate(current: viewer.current.as_ref(), guilds: guilds)
            }
            <div class="topbar-spacer"></div>
            match viewer.session {
                Some(true) => {
                    plan_chip(tier: viewer.tier)
                    account_menu(name: account_name(viewer.user.as_ref()))
                }
                Some(false) => {
                    <a href="/auth/discord" rel="external" class="btn btn-primary">
                        "Sign in"
                    </a>
                }
                None => {

                }
            }
        </div>
    }
    .boxed())
}

#[component]
async fn plan_chip(tier: Option<Tier>) -> Result<impl View> {
    Ok(view! {
        if let Some(tier) = tier {
            if tier == Tier::Free {
                <a
                    href="/upgrade"
                    class="plan-chip"
                    aria-label="Upgrade, current plan: Free"
                >
                    "Upgrade"
                </a>
            } else {
                <a href="/upgrade" class="plan-chip">
                    "Your plan: "
                    (tier.label())
                </a>
            }
        }
    })
}

#[must_use]
pub fn account_name(user: Option<&SessionUser>) -> &str {
    user.map_or(ACCOUNT_FALLBACK, |user| user.name.as_str())
}

#[component]
async fn account_menu(name: &str) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            class="topbar-button"
            popovertarget=(ACCOUNT_MENU_ID)
            aria-controls=(ACCOUNT_MENU_ID)
            aria-expanded="false"
            aria-label="Account menu"
        >
            icon(name: Icon::Users)
            <span class="account-name">(name)</span>
            icon(name: Icon::ChevronDown)
        </button>
        <div
            id=(ACCOUNT_MENU_ID)
            popover=""
            class="popover-panel menu-panel menu-panel-end"
        >
            <p class="menu-label"><strong>"Signed in with Discord"</strong></p>
            <a href="/upgrade" class="menu-item menu-plan">"Plans"</a>
            <a href="/logout" rel="external" class="menu-item">
                icon(name: Icon::LogOut)
                <span class="plate-name">"Log out"</span>
            </a>
        </div>
    })
}
