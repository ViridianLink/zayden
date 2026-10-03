#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::view::{Child, View, component, suspense, view};

use super::link::aria_current;
use super::sidebar::top_sidebar;
use crate::auth::check_session;
use crate::components::icons::{Icon, icon};
use crate::components::skeleton::{SkeletonShape, skeleton};
use crate::guild::dto::Tier;
use crate::guild::tier::get_user_tier;

#[component]
pub(super) async fn frame(
    sidebar: Child<'_>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class="app">
            app_navbar()
            <div class="app-body">
                (sidebar)
                <main class="app-main">(child)</main>
            </div>
        </div>
    })
}

/// The frame with the dashboard sidebar, for members pages outside a guild.
#[component]
pub async fn app_shell(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { frame(sidebar: view! { top_sidebar() }.into(), (child)) })
}

#[component]
async fn app_navbar(cx: &Cx) -> Result<impl View> {
    let location = uri(cx).path();
    let session = check_session(cx).await;

    Ok(view! {
        <nav class="app-navbar">
            <a
                href="/guilds"
                aria-current=(aria_current("/guilds", location))
                class="brand"
            >
                <span class="brand-mark">"Z"</span>
                "Zayden"
            </a>
            <div class="app-navbar-links">
                suspense(
                    fallback: view! { skeleton(shape: SkeletonShape::Badge) },
                    tier_badge()
                )
                match session {
                    Ok(true) => {
                        <a href="/logout" rel="external" class="btn btn-ghost">
                            icon(name: Icon::LogOut)
                            "Log out"
                        </a>
                    }
                    Ok(false) => {
                        <a href="/auth/discord" rel="external" class="btn btn-primary">
                            "Sign in"
                        </a>
                    }
                    Err(_) => {

                    }
                }
            </div>
        </nav>
    })
}

/// The signed-in user's tier, and an upgrade link for free users when one is
/// configured. Nothing when signed out or when the lookup fails.
#[component]
async fn tier_badge(cx: &Cx) -> Result<impl View> {
    let badge = get_user_tier(cx)
        .await
        .ok()
        .and_then(|info| info.tier.map(|tier| (tier, info.upgrade_url)));

    Ok(view! {
        if let Some((tier, upgrade_url)) = badge {
            <span class=(format!("tier-badge tier-{}", tier.css_suffix()))>
                (tier.label())
            </span>
            if let Some(url) = upgrade_url.filter(|_| tier == Tier::Free) {
                <a
                    href=(url)
                    class="btn-upgrade"
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    "Upgrade to Pro"
                </a>
            }
        }
    })
}
