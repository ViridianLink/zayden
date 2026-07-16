use leptos::prelude::*;

use super::icons::Icon;

#[component]
pub(crate) fn PublicLayout(children: Children) -> impl IntoView {
    view! {
        <div class="public">
            <header class="public-nav">
                <div class="public-nav-inner">
                    <a href="/" class="brand">
                        <span class="brand-mark">"Z"</span>
                        "Zayden"
                    </a>
                    <nav class="public-nav-links">
                        <a href="#features">"Features"</a>
                        <a href="/upgrade">"Pricing"</a>
                        <a href="/auth/discord" rel="external">"Login"</a>
                        <a href="/invite" rel="external" class="btn btn-primary">
                            <Icon name="plus"/>
                            "Add to Discord"
                        </a>
                    </nav>
                </div>
            </header>
            {children()}
            <Footer/>
        </div>
    }
}

#[component]
pub(crate) fn Footer() -> impl IntoView {
    view! {
        <footer class="footer">
            <div class="footer-inner">
                <span>"© 2026 Zayden. Not affiliated with Discord."</span>
                <div class="footer-links">
                    <a href="/invite" rel="external">"Invite"</a>
                    <a href="/upgrade">"Pricing"</a>
                    <a href="/auth/discord" rel="external">"Dashboard"</a>
                </div>
            </div>
        </footer>
    }
}
