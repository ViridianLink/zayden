use topcoat::Result;
use topcoat::router::page;
use topcoat::view::{View, view};

use super::layout::public_layout;
use crate::components::icons::{Icon, icon, module_icon, module_tint};

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

#[page("/")]
pub(crate) async fn landing() -> Result<impl View> {
    Ok(view! {
        public_layout(
            <main class="landing">
                <section class="hero">
                    <div class="hero-glow"></div>
                    <div class="hero-inner">
                        <span class="hero-eyebrow">
                            icon(name: Icon::Sparkles)
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
                                icon(name: Icon::Plus)
                                "Add to Discord"
                            </a>
                            <a href="/auth/discord" rel="external" class="btn btn-secondary btn-lg">
                                "Open Dashboard"
                                icon(name: Icon::ArrowRight)
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
                        #[key(feature.id)]
                        for feature in FEATURES {
                            let tint_style = format!("--tint: {}", module_tint(feature.id));
                            <div class="feature-card">
                                <div class="feature-icon" style=(tint_style)>
                                    icon(name: module_icon(feature.id))
                                </div>
                                <h3>(feature.title)</h3>
                                <p>(feature.desc)</p>
                            </div>
                        }
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
                            icon(name: Icon::Plus)
                            "Add to Discord"
                        </a>
                        <a href="/upgrade" class="btn btn-ghost btn-lg">"See Pro"</a>
                    </div>
                </section>
            </main>
        )
    })
}
