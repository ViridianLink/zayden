use leptos::prelude::*;

use super::icons::Icon;
use super::skeleton::Skeleton;
use crate::dto::SessionUser;
use crate::server::auth::current_session_user;

#[component]
pub(crate) fn PublicLayout(children: Children) -> impl IntoView {
    view! {
        <div class="public">
            <PublicNav/>
            {children()}
            <Footer/>
        </div>
    }
}

#[component]
fn PublicNav() -> AnyView {
    let session_user = Resource::new_blocking(|| (), |()| current_session_user());

    view! {
        <header class="public-nav">
            <div class="public-nav-inner">
                <a href="/" class="brand">
                    <span class="brand-mark">"Z"</span>
                    "Zayden"
                </a>
                <nav class="public-nav-links">
                    <a href="/#features" rel="external" class="public-nav-extra">"Features"</a>
                    <a href="/upgrade" class="public-nav-extra">"Pricing"</a>
                    <Suspense fallback=|| view! { <Skeleton class="skeleton-btn"/> }>
                        {move || {
                            session_user.get().and_then(Result::ok).map(|user| {
                                user.map_or_else(
                                    || view! {
                                        <a href="/auth/discord" rel="external">"Login"</a>
                                    }.into_any(),
                                    |user| view! { <PublicUser user=user/> }.into_any(),
                                )
                            })
                        }}
                    </Suspense>
                    <a href="/invite" rel="external" class="btn btn-primary" aria-label="Add to Discord">
                        <Icon name="plus"/>
                        <span class="public-nav-label">"Add to Discord"</span>
                    </a>
                </nav>
            </div>
        </header>
    }
    .into_any()
}

#[component]
fn PublicUser(user: SessionUser) -> impl IntoView {
    let avatar = user.avatar_url().map_or_else(
        || view! { <span class="public-user-avatar placeholder">{user.initial()}</span> }
            .into_any(),
        |url| view! {
            <img src=url alt="" width="24" height="24" class="public-user-avatar"/>
        }
        .into_any(),
    );

    view! {
        <span class="public-user">
            {avatar}
            <span class="public-user-name">{user.name}</span>
        </span>
        <a href="/guilds" class="btn btn-secondary" aria-label="My Servers">
            <Icon name="server"/>
            <span class="public-nav-label">"My Servers"</span>
        </a>
    }
}

#[component]
pub(crate) fn Footer() -> AnyView {
    view! {
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
    }
    .into_any()
}
