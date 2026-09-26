use leptos::prelude::*;
use leptos_meta::Title;

use crate::ui::components::legal::{ContactEmail, LegalDocument};

const UPDATED: &str = "26 September 2026";

#[component]
pub(crate) fn TermsPage() -> impl IntoView {
    view! {
        <Title text="Terms of Service - Zayden"/>
        <LegalDocument title="Terms of Service" updated=UPDATED>
            <Acceptance/>
            <Compliance/>
            <AcceptableUse/>
            <ServerAdmins/>
            <PaidFeatures/>
            <Warranty/>
            <Termination/>
            <Changes/>
            <GoverningLaw/>
        </LegalDocument>
    }
}

#[component]
fn Acceptance() -> impl IntoView {
    view! {
        <section id="acceptance">
            <h2>"Acceptance"</h2>
            <p class="legal-lead">
                "Zayden is a Discord bot and web dashboard operated by Oscar Six "
                "(\"ViridianLink\", \"we\", \"us\"). By using the bot or the dashboard "
                "you accept these terms and our "<a href="/privacy">"Privacy Policy"</a>"."
            </p>
            <p>
                <strong>
                    "By using the YouTube features, you agree to be bound by the "
                    <a href="https://www.youtube.com/t/terms">"YouTube Terms of Service"</a>
                    "."
                </strong>
                " Google's handling of data is covered by the "
                <a href="https://policies.google.com/privacy">"Google Privacy Policy"</a>
                "."
            </p>
        </section>
    }
}

#[component]
fn Compliance() -> impl IntoView {
    view! {
        <section id="discord">
            <h2>"Discord's rules"</h2>
            <p>
                "You must comply with Discord's "
                <a href="https://discord.com/terms">"Terms of Service"</a>
                " and "
                <a href="https://discord.com/guidelines">"Community Guidelines"</a>
                " whenever you use Zayden."
            </p>
        </section>
    }
}

#[component]
fn AcceptableUse() -> impl IntoView {
    view! {
        <section id="acceptable-use">
            <h2>"Acceptable use"</h2>
            <p>"You must not:"</p>
            <ul>
                <li>"use Zayden to harass, abuse or harm anyone;"</li>
                <li>"spam commands, messages or requests;"</li>
                <li>"try to disrupt, overload or break the bot, the dashboard or hosted servers;"</li>
                <li>
                    "get around cooldowns, usage limits, paid-feature checks or other "
                    "restrictions, or exploit bugs to do so;"
                </li>
                <li>"use Zayden for anything unlawful."</li>
            </ul>
        </section>
    }
}

#[component]
fn ServerAdmins() -> impl IntoView {
    view! {
        <section id="server-admins">
            <h2>"Server admins"</h2>
            <p>
                "If you manage a server, you are responsible for how Zayden is set up "
                "there. Only connect Patreon, YouTube and other accounts and channels "
                "that you own or are authorised to connect, and only turn on features "
                "such as AI replies where your members can expect them."
            </p>
        </section>
    }
}

#[component]
fn PaidFeatures() -> impl IntoView {
    view! {
        <section id="paid-features">
            <h2>"Paid features"</h2>
            <p>
                "Pro memberships can be bought on Ko-fi or through Discord, and Ultra "
                "through Discord. "
                "Payments are handled by Ko-fi or Discord under their own terms. A "
                "membership's features stay active while it is paid up and end when "
                "it lapses or is cancelled. Refunds are handled case by case at our "
                "discretion, and you should not expect one."
            </p>
            <h3>"Game-server hosting"</h3>
            <ul>
                <li>
                    "A new hosted server starts with a short free trial (currently four "
                    "hours) so you can check that it works. If it is not paid for when "
                    "the trial ends, it is suspended and deleted."
                </li>
                <li>
                    "Paid servers are billed monthly through Ko-fi. Some memberships "
                    "include a server at no extra cost while the membership is active."
                </li>
                <li>
                    "We send a reminder by Discord direct message before a subscription "
                    "lapses. If it lapses, the server is suspended and deleted after a "
                    "grace period (currently three days). Paying during the grace period "
                    "restores it."
                </li>
                <li>
                    "Cancelling a paid server suspends it and deletes it after the same "
                    "grace period; cancelling a trial deletes it shortly afterwards."
                </li>
                <li>
                    "Deleted servers and their files cannot be recovered. Keep your own "
                    "backups of anything you need."
                </li>
            </ul>
        </section>
    }
}

#[component]
fn Warranty() -> impl IntoView {
    view! {
        <section id="warranty">
            <h2>"No warranty and limited liability"</h2>
            <p>
                "Zayden is provided \"as is\" and \"as available\", without warranties "
                "of any kind. We do not guarantee that it will always be available, "
                "error-free or that data will never be lost, and features may change "
                "or be removed."
            </p>
            <p>
                "To the fullest extent the law allows, we are not liable for indirect "
                "or consequential losses, or for loss of data, arising from your use of "
                "Zayden. Nothing in these terms limits liability that cannot be "
                "limited by law."
            </p>
        </section>
    }
}

#[component]
fn Termination() -> impl IntoView {
    view! {
        <section id="termination">
            <h2>"Termination"</h2>
            <p>
                "We may suspend or withdraw access to Zayden, the dashboard or hosted "
                "servers for anyone who abuses the service or breaks these terms. You "
                "can stop using Zayden at any time by removing it from your server and "
                "logging out of the dashboard."
            </p>
        </section>
    }
}

#[component]
fn Changes() -> impl IntoView {
    view! {
        <section id="changes">
            <h2>"Changes and contact"</h2>
            <p>
                "We may update these terms. When we do, we change this page and the "
                "\"Last updated\" date at the top; continuing to use Zayden after that "
                "means you accept the new terms. Questions: "<ContactEmail/>"."
            </p>
        </section>
    }
}

#[component]
fn GoverningLaw() -> impl IntoView {
    view! {
        <section id="governing-law">
            <h2>"Governing law"</h2>
            <p>"These terms are governed by the laws of the United Kingdom."</p>
        </section>
    }
}
