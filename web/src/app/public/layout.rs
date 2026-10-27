#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{Child, View, component, view};

use crate::auth::{self, AuthError};
use crate::components::brand::brand_mark;
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
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
        <div class="flex flex-col min-h-screen">
            public_nav(session: session, location: location)
            <main id="main" class="public-main" tabindex="-1">(child)</main>
            footer(session: session)
        </div>
    })
}

#[component]
pub async fn public_nav(
    cx: &Cx,
    session: SessionSlot,
    #[default("/")] location: &str,
) -> Result<impl View> {
    Ok(view! {
        <header
            class="sticky top-0 z-100 flex items-center justify-center h-(--topbar-h) py-0 px-4 bg-bg border-b border-solid border-b-rule"
        >
            <div
                class="w-full max-w-(--content-max) flex items-center justify-between gap-4"
            >
                <a class="brand" (link_attrs(cx, "/", prefetch_mode(cx)))>
                    brand_mark()
                    <span class="max-md:hidden">"Zayden"</span>
                </a>
                <nav class="flex items-center gap-2 max-md:hidden" aria-label="Main">
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
                role="dialog"
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
async fn public_links(
    cx: &Cx,
    session: SessionSlot,
    location: &str,
) -> Result<impl View> {
    let pricing = (location == "/upgrade").then_some("page");

    Ok(view! {
        <a href="/#features" rel="external" class="public-nav-link">"Features"</a>
        <a
            class="public-nav-link"
            aria-current=(pricing)
            (link_attrs(cx, "/upgrade", prefetch_mode(cx)))
        >
            "Pricing"
        </a>
        match session {
            SessionSlot::Unavailable => {

            }
            SessionSlot::SignedOut => {
                <a
                    href="/auth/discord"
                    rel="external"
                    class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md))
                >
                    "Sign in"
                </a>
            }
            SessionSlot::SignedIn => {
                <a
                    class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
                    (link_attrs(cx, "/guilds", prefetch_mode(cx)))
                >
                    "Open dashboard"
                </a>
            }
        }
    })
}

#[component]
async fn footer(cx: &Cx, session: SessionSlot) -> Result<impl View> {
    Ok(view! {
        <footer
            class="border-t border-solid border-t-rule py-6 px-4 text-text-2 text-[13px]"
        >
            <div
                class="w-full max-w-(--content-max) flex flex-wrap items-center justify-between gap-4"
            >
                <span>"© 2026 Zayden. Not affiliated with Discord."</span>
                <nav class="flex flex-wrap gap-y-1 gap-x-4" aria-label="Footer">
                    <a
                        href="/invite"
                        rel="external"
                        class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]"
                    >
                        "Invite"
                    </a>
                    <a
                        class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]"
                        (link_attrs(cx, "/upgrade", prefetch_mode(cx)))
                    >
                        "Pricing"
                    </a>
                    match session {
                        SessionSlot::SignedIn => {
                            <a
                                class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]"
                                (link_attrs(cx, "/guilds", prefetch_mode(cx)))
                            >
                                "Dashboard"
                            </a>
                        }
                        SessionSlot::SignedOut | SessionSlot::Unavailable => {
                            <a
                                href="/auth/discord"
                                rel="external"
                                class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]"
                            >
                                "Sign in"
                            </a>
                        }
                    }
                    <a
                        class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]"
                        (link_attrs(cx, "/privacy", prefetch_mode(cx)))
                    >
                        "Privacy Policy"
                    </a>
                    <a
                        class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]"
                        (link_attrs(cx, "/terms", prefetch_mode(cx)))
                    >
                        "Terms of Service"
                    </a>
                </nav>
            </div>
        </footer>
    })
}
