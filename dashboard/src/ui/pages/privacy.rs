use leptos::prelude::*;
use leptos_meta::Title;

use crate::ui::components::legal::{ContactEmail, LegalDocument};

const UPDATED: &str = "26 September 2026";

#[component]
pub(crate) fn PrivacyPage() -> impl IntoView {
    view! {
        <Title text="Privacy Policy - Zayden"/>
        <LegalDocument title="Privacy Policy" updated=UPDATED>
            <Intro/>
            <DiscordData/>
            <FeatureData/>
            <AiFeatures/>
            <Payments/>
            <Patreon/>
            <Youtube/>
            <JellyfinWatch/>
            <Palworld/>
            <ThirdParties/>
            <Security/>
            <Retention/>
            <Rights/>
            <Minors/>
            <Changes/>
        </LegalDocument>
    }
}

#[component]
fn Intro() -> impl IntoView {
    view! {
        <p class="legal-lead">
            "Zayden is a Discord bot and web dashboard operated by Oscar Six "
            "(\"ViridianLink\", \"we\", \"us\"). This policy explains what information "
            "Zayden collects, why it is used, who it is shared with, and the choices "
            "you have. It covers the bot and the dashboard at zayden.viridian.icu."
        </p>
        <p>"Questions or requests about your data: "<ContactEmail/>"."</p>
        <p>
            "We do not sell personal information, and we do not use it for "
            "advertising. The dashboard sets no analytics or advertising cookies."
        </p>
    }
}

#[component]
fn DiscordData() -> impl IntoView {
    view! {
        <section id="discord">
            <h2>"Information from Discord"</h2>
            <h3>"Identifiers"</h3>
            <p>
                "To work at all, Zayden stores Discord user IDs, server (guild) IDs and "
                "the channel, role and message IDs a feature needs. Some features also "
                "store a username or display name, for example moderation records."
            </p>
            <h3>"Dashboard sign-in"</h3>
            <p>
                "You sign in to the dashboard with Discord. Zayden asks Discord for your "
                "profile, your server list and permission to update slash-command "
                "permissions in servers you manage. After sign-in we store:"
            </p>
            <ul>
                <li>
                    "a random session token, kept in an HttpOnly cookie in your "
                    "browser;"
                </li>
                <li>
                    "a matching session record holding your Discord user ID, the Discord "
                    "access token issued at sign-in (used to read your server list and "
                    "to update command permissions on your behalf) and an expiry time "
                    "seven days later."
                </li>
            </ul>
            <p>
                "Your server list is cached in memory for about a minute so pages load "
                "quickly. The sign-in request includes Discord's email scope, but "
                "Zayden does not store your Discord email address."
            </p>
            <h3>"Server settings"</h3>
            <p>
                "Server admins configure Zayden per server: which modules are on, the "
                "channels and roles each feature uses, and any text they write for the "
                "bot to post, such as greetings, rules or FAQ articles."
            </p>
        </section>
    }
}

#[component]
fn FeatureData() -> impl IntoView {
    view! {
        <section id="features">
            <h2>"Data stored by each feature"</h2>
            <p>
                "Zayden only stores data for the features a server uses. Zayden does "
                "not keep a copy of ordinary chat messages."
            </p>
            <ul>
                <li>
                    <strong>"Levels: "</strong>
                    "XP, level, message count and the time XP was last awarded, per "
                    "server. The count is kept, not the messages."
                </li>
                <li>
                    <strong>"Economy and games: "</strong>
                    "coin and gem balances, inventory, active items, goals, mining "
                    "progress, game statistics and daily or gift cooldowns."
                </li>
                <li>
                    <strong>"Gold stars and bingo: "</strong>
                    "star counts, and your daily bingo card."
                </li>
                <li>
                    <strong>"Family: "</strong>
                    "partner, parent, child and block relationships within a server."
                </li>
                <li>
                    <strong>"Support tickets and FAQ: "</strong>
                    "ticket thread IDs, the opener and helper of a thread and activity "
                    "times used for idle and stale reminders, links saved by helpers, "
                    "and FAQ articles written by admins or generated from solved "
                    "tickets (see AI features)."
                </li>
                <li>
                    <strong>"Suggestions: "</strong>
                    "only the channels and vote thresholds; suggestions themselves stay "
                    "as Discord messages."
                </li>
                <li>
                    <strong>"Reaction roles: "</strong>
                    "which emoji on which message grants which role."
                </li>
                <li>
                    <strong>"Greetings: "</strong>
                    "the morning and night messages, image links and cooldowns a server "
                    "sets."
                </li>
                <li>
                    <strong>"Looking for group: "</strong>
                    "your posts (activity, description, start time, group size), who "
                    "joined, and the time zone you set."
                </li>
                <li>
                    <strong>"Temporary voice channels: "</strong>
                    "the channel owner, trusted and invited members, the channel "
                    "password if you set one, and whether the channel persists."
                </li>
                <li>
                    <strong>"Honeypot moderation: "</strong>
                    "the decoy channel and its settings. Anyone who posts there is "
                    "soft-banned (banned and unbanned) to remove their recent messages; "
                    "Zayden keeps no record of it beyond Discord's audit log."
                </li>
                <li>
                    <strong>"Moderation: "</strong>
                    "infractions: the member's ID and username, the moderator's ID and "
                    "username, the type, points, reason and date."
                </li>
                <li>
                    <strong>"Music: "</strong>
                    "server music settings (DJ role, volume, auto-disconnect, 24/7 and "
                    "autoplay, announcement channel). The queue is held in memory only."
                </li>
                <li>
                    <strong>"Verification and roles: "</strong>
                    "the verified role and other role settings."
                </li>
            </ul>
        </section>
    }
}

#[component]
fn AiFeatures() -> impl IntoView {
    view! {
        <section id="ai">
            <h2>"AI features"</h2>
            <p>
                "Some features send text to an AI provider through an "
                "OpenRouter-compatible API (currently OpenRouter), which forwards it "
                "to the model that answers. Zayden does not store these conversations."
            </p>
            <ul>
                <li>
                    <strong>"Chat replies: "</strong>
                    "only in channels where a server admin has turned AI on, and only "
                    "when you mention the bot. Zayden sends your message, the messages "
                    "it replies to, and the display names of their authors (and of "
                    "anyone mentioned)."
                </li>
                <li>
                    <strong>"Support triage and answers: "</strong>
                    "when a server enables it, the title and opening post of a new "
                    "support ticket, any screenshots attached to it and questions "
                    "asked of the FAQ are sent so the bot can suggest an answer."
                </li>
                <li>
                    <strong>"FAQ generation: "</strong>
                    "when a server enables it, the transcript of a solved ticket is sent "
                    "to draft an FAQ article. Speakers are replaced with anonymous "
                    "labels, and email addresses, IP addresses, Discord IDs, mentions "
                    "and API tokens are redacted first. The resulting article is stored "
                    "for that server."
                </li>
                <li>
                    <strong>"Watch recommendations: "</strong>
                    "the mood or description you type is sent to turn it into search "
                    "filters."
                </li>
            </ul>
        </section>
    }
}

#[component]
fn Payments() -> impl IntoView {
    view! {
        <section id="payments">
            <h2>"Payments and entitlements"</h2>
            <p>
                "Payments are made on Ko-fi or through Discord. Zayden never sees your "
                "card or bank details."
            </p>
            <h3>"Ko-fi"</h3>
            <ul>
                <li>
                    "To link a Ko-fi membership, you enter the email you use on Ko-fi. "
                    "Zayden stores only a SHA-256 hash of that address with your "
                    "Discord user ID, never the address itself."
                </li>
                <li>
                    "Ko-fi notifies Zayden of subscription payments. The email in the "
                    "notification is hashed to find your account and is not stored. "
                    "We record the resulting entitlement: the Ko-fi transaction ID, "
                    "tier, and when it was granted and expires."
                </li>
                <li>
                    "For game-server hosting payments we also keep the payment's "
                    "reference, tier name, amount, currency and which server it paid "
                    "for. A claim code in the payment message is read to match it; the "
                    "message itself is not stored."
                </li>
            </ul>
            <h3>"Discord purchases"</h3>
            <p>
                "If you buy premium through Discord, Discord tells Zayden about the "
                "entitlement and we record the tier, the user or server it applies to, "
                "and its dates."
            </p>
            <h3>"Game-server hosting"</h3>
            <p>
                "When you start a hosted game server, Zayden creates an account for you "
                "on the Pelican game-server panel we host ourselves. The account gets a "
                "username, "
                "your Discord display name, and a placeholder email address built from "
                "your Discord ID (not your real email). Its password is generated, sent "
                "to you in a Discord direct message, and not stored by Zayden."
            </p>
            <p>
                "We keep a record of each server: owner, game, plan, resources, price, "
                "billing state, trial and paid-until dates, claim code and address."
            </p>
        </section>
    }
}

#[component]
fn Patreon() -> impl IntoView {
    view! {
        <section id="patreon">
            <h2>"Patreon connection"</h2>
            <p>
                "A server admin can connect a Patreon creator account so new posts are "
                "announced in Discord. Zayden asks Patreon for the creator's identity, "
                "campaign and posts, and permission to register a webhook."
            </p>
            <ul>
                <li>
                    "Stored: the campaign ID, creator name, OAuth access and refresh "
                    "tokens (so new posts can be read), the webhook ID and secret, and "
                    "who connected it."
                </li>
                <li>
                    "Cached for announcements: each post's ID, title, link, content, "
                    "thumbnail, whether it is public, and when it was published and "
                    "announced."
                </li>
                <li>
                    "Disconnecting from the dashboard removes the webhook from Patreon "
                    "and deletes the connection, including the tokens. Once no server "
                    "is connected to the campaign, its cached posts are deleted too. "
                    "The server's announcement settings are kept."
                </li>
            </ul>
        </section>
    }
}

#[component]
fn Youtube() -> impl IntoView {
    view! {
        <section id="youtube">
            <h2>"YouTube connection"</h2>
            <p>
                <strong>"Zayden uses YouTube API Services."</strong>
                " A server admin can connect a YouTube channel so its new uploads are "
                "announced in a Discord channel they choose."
            </p>
            <h3>"When you connect"</h3>
            <p>
                "You sign in with Google and grant read-only YouTube access. Zayden "
                "makes a single request, "<code>"channels.list?mine=true"</code>", and "
                "reads only your channel ID, channel title and uploads-playlist ID."
            </p>
            <p>
                <strong>
                    "The Google access token is revoked immediately after that request "
                    "and is never stored."
                </strong>
                " Zayden cannot read any private data from your Google account "
                "afterwards."
            </p>
            <h3>"After you connect"</h3>
            <p>
                "Zayden reads only public upload data for the channel, using its own "
                "API key rather than your credentials. It also subscribes to YouTube's "
                "push notifications so it hears about new uploads quickly."
            </p>
            <h3>"What is stored"</h3>
            <ul>
                <li>"The channel ID, title and uploads-playlist ID."</li>
                <li>"Public video IDs, titles and publish times, and when each was announced."</li>
                <li>"The push-notification subscription secret and its expiry."</li>
                <li>
                    "Which Discord server connected the channel, who connected it, and "
                    "the announcement channel."
                </li>
            </ul>
            <h3>"Disconnecting and revoking"</h3>
            <p>
                "Disconnecting from the dashboard removes the server's connection and "
                "stops announcements. Once no server uses the channel, Zayden "
                "unsubscribes from its push notifications and deletes its stored video "
                "list. The channel's ID, title, uploads-playlist ID and subscription "
                "secret are kept so it can be reconnected, until deletion is requested "
                "at "<ContactEmail/>". You can also revoke Zayden's access at any time "
                "at "
                <a href="https://security.google.com/settings/security/permissions">
                    "https://security.google.com/settings/security/permissions"
                </a>
                "."
            </p>
            <p>
                "Google's handling of your data is described in the "
                <a href="https://policies.google.com/privacy">"Google Privacy Policy"</a>
                "."
            </p>
            <p class="legal-callout">
                "Zayden's use and transfer of information received from Google APIs will "
                "adhere to the "
                <a href="https://developers.google.com/terms/api-services-user-data-policy">
                    "Google API Services User Data Policy"
                </a>
                ", including the Limited Use requirements."
            </p>
        </section>
    }
}

#[component]
fn JellyfinWatch() -> impl IntoView {
    view! {
        <section id="watch">
            <h2>"Jellyfin and Watch"</h2>
            <p>
                "These features use a Jellyfin media server and its Jellyseerr request "
                "service, both run by us."
            </p>
            <ul>
                <li>
                    <strong>"Linked accounts: "</strong>
                    "your Discord ID with your Jellyfin user ID and username, your "
                    "Jellyseerr user ID, whether your watch streak is public (it is "
                    "private by default), and any Letterboxd or Serializd username you "
                    "add. Unlinking deletes the link."
                </li>
                <li>
                    <strong>"Watch parties: "</strong>
                    "the host, title, schedule and channel. Guests get a temporary "
                    "Jellyfin account that is deleted after the party."
                </li>
                <li>
                    <strong>"Games: "</strong>
                    "rounds played, who answered, and each player's points."
                </li>
                <li>
                    <strong>"Playback stats: "</strong>
                    "seconds watched and items played per day, read from the Jellyfin "
                    "server, for streaks and statistics."
                </li>
                <li>
                    <strong>"Letterboxd and Serializd: "</strong>
                    "if you add a username, Zayden reads that account's public diary "
                    "from those sites."
                </li>
                <li>
                    <strong>"Content warnings: "</strong>
                    "film and show titles are looked up on DoesTheDogDie; no personal "
                    "data is sent."
                </li>
            </ul>
        </section>
    }
}

#[component]
fn Palworld() -> impl IntoView {
    view! {
        <section id="palworld">
            <h2>"Palworld"</h2>
            <ul>
                <li>
                    <strong>"Player links: "</strong>
                    "your Discord ID with your Palworld player ID, in-game name and the "
                    "host you play on."
                </li>
                <li>
                    <strong>"Save uploads: "</strong>
                    "save files you upload are stored on our server with an expiry "
                    "time, and the file and its record are deleted automatically once "
                    "it passes."
                </li>
            </ul>
        </section>
    }
}

#[component]
fn ThirdParties() -> impl IntoView {
    view! {
        <section id="third-parties">
            <h2>"Third parties"</h2>
            <p>"Zayden shares data only as needed to provide each feature:"</p>
            <ul>
                <li>
                    <strong>"Discord: "</strong>
                    "the platform Zayden runs on; everything the bot does goes through "
                    "Discord's API. "
                    <a href="https://discord.com/privacy">"Discord Privacy Policy"</a>
                    "."
                </li>
                <li>
                    <strong>"OpenRouter and the AI model provider: "</strong>
                    "the text described under AI features."
                </li>
                <li>
                    <strong>"Patreon: "</strong>
                    "to read a connected creator's posts and manage its webhook."
                </li>
                <li>
                    <strong>"Google / YouTube: "</strong>
                    "to verify channel ownership and read public uploads, as described "
                    "above."
                </li>
                <li>
                    <strong>"Ko-fi: "</strong>
                    "processes membership and hosting payments and notifies Zayden of "
                    "them."
                </li>
                <li>
                    <strong>"Music sources: "</strong>
                    "the search terms or links you request are sent to the service that "
                    "hosts the audio, such as "
                    // "YouTube, "
                    "Spotify or an internet radio station."
                </li>
                <li>
                    <strong>"Letterboxd and Serializd: "</strong>
                    "only the username you choose to add."
                </li>
                <li>
                    <strong>"DoesTheDogDie: "</strong>
                    "titles only, no personal data."
                </li>
                <li>
                    <strong>"Cloudflare: "</strong>
                    "hosting and the network tunnel that serves the dashboard, so it "
                    "handles web traffic to and from the site."
                </li>
            </ul>
        </section>
    }
}

#[component]
fn Security() -> impl IntoView {
    view! {
        <section id="security">
            <h2>"Storage and security"</h2>
            <p>
                "Data is kept in a self-hosted PostgreSQL database, and the Jellyfin "
                "media server and Pelican game-server panel also run on infrastructure "
                "we operate. API keys, client "
                "secrets and other credentials are kept in server-side configuration "
                "and are never sent to your browser. Session cookies are HttpOnly and "
                "sent only over HTTPS, and Ko-fi emails are stored only as hashes. No "
                "system is perfectly secure, but we limit what we keep to what each "
                "feature needs."
            </p>
        </section>
    }
}

#[component]
fn Retention() -> impl IntoView {
    view! {
        <section id="retention">
            <h2>"How long data is kept"</h2>
            <ul>
                <li>
                    "Dashboard sessions expire after seven days and expired sessions "
                    "are deleted every hour. Logging out deletes your session at once."
                </li>
                <li>"The cached server list lasts about a minute."</li>
                <li>"The Google access token is revoked and discarded at connect time."</li>
                <li>
                    "A YouTube channel's stored video list, and a Patreon campaign's "
                    "cached posts, are deleted once no server is connected to them."
                </li>
                <li>"Palworld save uploads are deleted when they expire."</li>
                <li>"Watch-party guest accounts are deleted after the party."</li>
                <li>
                    "Hosted game servers are deleted when an unpaid trial ends, or after "
                    "the grace period once a subscription lapses."
                </li>
                <li>
                    "When Zayden is removed from a server, that server's settings and "
                    "feature data are kept for 30 days, so everything comes back if "
                    "Zayden is added again. After 30 days they are deleted "
                    "automatically, along with the server's Patreon and YouTube "
                    "connections. If one of Zayden's sibling bots is still in the "
                    "server, the 30 days start when the last of them leaves."
                </li>
                <li>
                    "Everything else is kept until deletion is requested. This "
                    "includes data that belongs to you rather than to a server, such "
                    "as economy balances and linked accounts, and looking-for-group "
                    "posts and temporary voice channel records, which are not linked "
                    "to a server in our database."
                </li>
            </ul>
        </section>
    }
}

#[component]
fn Rights() -> impl IntoView {
    view! {
        <section id="rights">
            <h2>"Your choices and rights"</h2>
            <ul>
                <li>
                    "To access, correct or delete your data, email "<ContactEmail/>
                    " from an address we can reply to and include your Discord user ID."
                </li>
                <li>
                    "Server admins can disconnect Patreon and YouTube, turn modules and "
                    "AI replies off, and change settings from the dashboard at any time."
                </li>
                <li>
                    "You can unlink your Jellyfin account, make your watch streak "
                    "private, and log out of the dashboard yourself."
                </li>
                <li>
                    "Depending on where you live you may have further rights, such as "
                    "to object to processing or to complain to your data protection "
                    "authority."
                </li>
            </ul>
        </section>
    }
}

#[component]
fn Minors() -> impl IntoView {
    view! {
        <section id="children">
            <h2>"Children"</h2>
            <p>
                "Zayden follows Discord's minimum age requirements and is not directed "
                "at children under 13. If you believe a child under 13 has given us "
                "information, contact us and we will delete it."
            </p>
        </section>
    }
}

#[component]
fn Changes() -> impl IntoView {
    view! {
        <section id="changes">
            <h2>"Changes to this policy"</h2>
            <p>
                "When this policy changes, we update this page and the \"Last updated\" "
                "date at the top."
            </p>
        </section>
    }
}
