#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::view::{Child, View, component, view};

use crate::auth::{self, AuthError, SessionUser};
use crate::components::icons::{Icon, icon};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionSlot {
    /// The session or profile lookup failed: the slot stays empty.
    Unavailable,
    SignedOut,
    SignedIn(SessionUser),
}

impl From<Result<Option<SessionUser>, AuthError>> for SessionSlot {
    fn from(lookup: Result<Option<SessionUser>, AuthError>) -> Self {
        match lookup {
            Ok(Some(user)) => Self::SignedIn(user),
            Ok(None) => Self::SignedOut,
            Err(_) => Self::Unavailable,
        }
    }
}

/// Resolves the visitor's Discord profile on every render, so a signed-in
/// visitor costs one Discord `/users/@me` request per page load.
#[component]
pub async fn public_layout(
    cx: &Cx,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let session = SessionSlot::from(auth::current_session_user(cx).await);

    Ok(view! {
        <div class="public">
            public_nav(session: session)
            (child)
            footer()
        </div>
    })
}

#[component]
pub async fn public_nav(session: SessionSlot) -> Result<impl View> {
    Ok(view! {
        <header class="public-nav">
            <div class="public-nav-inner">
                <a href="/" class="brand">
                    <span class="brand-mark">"Z"</span>
                    "Zayden"
                </a>
                <nav class="public-nav-links">
                    <a href="/#features" rel="external" class="public-nav-extra">"Features"</a>
                    <a href="/upgrade" class="public-nav-extra">"Pricing"</a>
                    match session {
                        SessionSlot::Unavailable => {},
                        SessionSlot::SignedOut => {
                            <a href="/auth/discord" rel="external">"Login"</a>
                        },
                        SessionSlot::SignedIn(user) => public_user(user: user),
                    }
                    <a href="/invite" rel="external" class="btn btn-primary" aria-label="Add to Discord">
                        icon(name: Icon::Plus)
                        <span class="public-nav-label">"Add to Discord"</span>
                    </a>
                </nav>
            </div>
        </header>
    })
}

#[component]
async fn public_user(user: SessionUser) -> Result<impl View> {
    let avatar = user.avatar_url();
    let initial = user.initial();

    Ok(view! {
        <span class="public-user">
            if let Some(url) = avatar {
                <img src=(url) alt="" width="24" height="24" class="public-user-avatar">
            } else {
                <span class="public-user-avatar placeholder">(initial)</span>
            }
            <span class="public-user-name">(user.name)</span>
        </span>
        <a href="/guilds" class="btn btn-secondary" aria-label="My Servers">
            icon(name: Icon::Server)
            <span class="public-nav-label">"My Servers"</span>
        </a>
    })
}

#[component]
async fn footer() -> Result<impl View> {
    Ok(view! {
        <footer class="footer">
            <div class="footer-inner">
                <span>"© 2026 Zayden. Not affiliated with Discord."</span>
                <div class="footer-links">
                    <a href="/invite" rel="external">"Invite"</a>
                    <a href="/upgrade">"Pricing"</a>
                    <a href="/auth/discord" rel="external">"Dashboard"</a>
                    <a href="/privacy">"Privacy Policy"</a>
                    <a href="/terms">"Terms of Service"</a>
                </div>
            </div>
        </footer>
    })
}
