#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::view::{Child, View, component, view};

use crate::auth::{self, AuthError};
use crate::components::icons::{Icon, icon};
use crate::components::progress_bar::progress_bar;

const MENU_ID: &str = "public-menu";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionSlot {
    Unavailable,
    SignedOut,
    SignedIn,
}

impl From<Result<bool, AuthError>> for SessionSlot {
    fn from(lookup: Result<bool, AuthError>) -> Self {
        match lookup {
            Ok(true) => Self::SignedIn,
            Ok(false) => Self::SignedOut,
            Err(_) => Self::Unavailable,
        }
    }
}

#[component]
pub async fn public_layout(
    cx: &Cx,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let session = SessionSlot::from(auth::check_session(cx).await);
    let location = uri(cx).path();

    Ok(view! {
        <div class="public">
            public_nav(session: session, location: location)
            <main id="main" class="public-main" tabindex="-1">(child)</main>
            footer(session: session)
        </div>
    })
}

#[component]
pub async fn public_nav(
    session: SessionSlot,
    #[default("/")] location: &str,
) -> Result<impl View> {
    Ok(view! {
        <header class="public-header">
            <div class="public-header-inner">
                <a href="/" class="brand">
                    <span class="brand-mark" aria-hidden="true">"Z"</span>
                    <span class="brand-name">"Zayden"</span>
                </a>
                <nav class="public-nav-links" aria-label="Main">
                    public_links(session: session, location: location)
                </nav>
                <button
                    type="button"
                    class="topbar-button public-menu-button"
                    popovertarget=(MENU_ID)
                    aria-controls=(MENU_ID)
                    aria-expanded="false"
                    aria-label="Menu"
                >
                    icon(name: Icon::Menu)
                </button>
            </div>
            <div
                id=(MENU_ID)
                popover=""
                class="sheet-panel sheet-end"
                aria-label="Menu"
            >
                <div class="sheet-head">
                    <span class="label">"Menu"</span>
                    <button
                        type="button"
                        class="topbar-button"
                        popovertarget=(MENU_ID)
                        popovertargetaction="hide"
                        aria-label="Close menu"
                        autofocus=""
                    >
                        icon(name: Icon::X)
                    </button>
                </div>
                <nav class="sheet-body" aria-label="Main">
                    public_links(session: session, location: location)
                </nav>
            </div>
            progress_bar()
        </header>
    })
}

#[component]
async fn public_links(session: SessionSlot, location: &str) -> Result<impl View> {
    let pricing = (location == "/upgrade").then_some("page");

    Ok(view! {
        <a href="/#features" rel="external" class="public-nav-link">"Features"</a>
        <a href="/upgrade" class="public-nav-link" aria-current=(pricing)>"Pricing"</a>
        match session {
            SessionSlot::Unavailable => {

            }
            SessionSlot::SignedOut => {
                <a href="/auth/discord" rel="external" class="btn btn-secondary">
                    "Sign in"
                </a>
            }
            SessionSlot::SignedIn => {
                <a href="/guilds" class="btn btn-primary">"Open dashboard"</a>
            }
        }
    })
}

#[component]
async fn footer(session: SessionSlot) -> Result<impl View> {
    Ok(view! {
        <footer class="footer">
            <div class="footer-inner">
                <span>"© 2026 Zayden. Not affiliated with Discord."</span>
                <nav class="footer-links" aria-label="Footer">
                    <a href="/invite" rel="external">"Invite"</a>
                    <a href="/upgrade">"Pricing"</a>
                    match session {
                        SessionSlot::SignedIn => <a href="/guilds">"Dashboard"</a>,
                        SessionSlot::SignedOut | SessionSlot::Unavailable => {
                            <a href="/auth/discord" rel="external">"Sign in"</a>
                        }
                    }
                    <a href="/privacy">"Privacy Policy"</a>
                    <a href="/terms">"Terms of Service"</a>
                </nav>
            </div>
        </footer>
    })
}
