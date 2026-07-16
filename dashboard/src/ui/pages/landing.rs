use leptos::prelude::*;
use leptos_meta::Title;

use crate::ui::components::icons::{Icon, module_icon, module_tint};
use crate::ui::components::public_layout::PublicLayout;

struct Feature {
    id: &'static str,
    title: &'static str,
    desc: &'static str,
}

const FEATURES: &[Feature] = &[
    Feature {
        id: "music",
        title: "Music",
        desc: "High-quality voice playback with queue controls and 24/7 mode.",
    },
    Feature {
        id: "gambling",
        title: "Economy & Games",
        desc: "Currency, a shop, leaderboards, and a dozen mini-games to play.",
    },
    Feature {
        id: "family",
        title: "Family",
        desc: "Marriage, adoption, and a full family tree for your community.",
    },
    Feature {
        id: "palworld",
        title: "Palworld",
        desc: "Save parsing, a breeding solver, and world sync for your server.",
    },
    Feature {
        id: "ticket",
        title: "Tickets & Support",
        desc: "Support tickets and FAQ panels to keep your mod team organised.",
    },
    Feature {
        id: "marathon",
        title: "Marathon",
        desc: "Wiki lookups and news for the games your members care about.",
    },
];

#[component]
pub(crate) fn LandingPage() -> impl IntoView {
    view! {
        <Title text="Zayden — the all-in-one Discord bot"/>
        <PublicLayout>
            <main class="landing">
                <section class="hero">
                    <div class="hero-glow"></div>
                    <div class="hero-inner">
                        <span class="hero-eyebrow">
                            <Icon name="sparkles"/>
                            "One bot. Every module."
                        </span>
                        <h1 class="hero-title">
                            "The Discord bot that "
                            <span class="accent-text">"grows with your server"</span>
                        </h1>
                        <p class="hero-subtitle">
                            "Music, economy, moderation tools and more — configured from a "
                            "clean dashboard, enforced natively by Discord. Free to run, "
                            "Pro only where it costs us."
                        </p>
                        <div class="hero-actions">
                            <a href="/invite" rel="external" class="btn btn-primary btn-lg">
                                <Icon name="plus"/>
                                "Add to Discord"
                            </a>
                            <a href="/auth/discord" rel="external" class="btn btn-secondary btn-lg">
                                "Open Dashboard"
                                <Icon name="arrow-right"/>
                            </a>
                        </div>
                    </div>
                </section>

                <section id="features" class="landing-section landing-block">
                    <div class="section-head">
                        <h2>"Everything your community needs"</h2>
                        <p>
                            "Toggle modules on or off per server. Each one plugs straight "
                            "into Discord's command-permission system."
                        </p>
                    </div>
                    <div class="feature-grid">
                        {FEATURES.iter().map(|f| {
                            let tint_style = format!("--tint: {}", module_tint(f.id));
                            view! {
                                <div class="feature-card">
                                    <div class="feature-icon" style=tint_style>
                                        <Icon name=module_icon(f.id)/>
                                    </div>
                                    <h3>{f.title}</h3>
                                    <p>{f.desc}</p>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </section>

                <section class="cta-band">
                    <h2>"Ready in under a minute"</h2>
                    <p>
                        "Invite Zayden, sign in with Discord, and start configuring your "
                        "server from the dashboard."
                    </p>
                    <div class="hero-actions">
                        <a href="/invite" rel="external" class="btn btn-primary btn-lg">
                            <Icon name="plus"/>
                            "Add to Discord"
                        </a>
                        <a href="/upgrade" class="btn btn-ghost btn-lg">
                            "See Pro"
                        </a>
                    </div>
                </section>
            </main>
        </PublicLayout>
    }
}
