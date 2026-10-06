#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::page;
use topcoat::runtime::{Event, expr, signal};
use topcoat::view::{View, component, view};

use crate::admin::{AdminError, list_bot_guilds, parse_guild_id};
use crate::components::guild_grid::GuildCard;
use crate::shell::app_shell;
use crate::util::server_error_text;

const SEPARATOR: &str = "\n";

fn matching(names: &str, needle: &str) -> f64 {
    if names.is_empty() {
        return 0.0;
    }
    let count = names.split(SEPARATOR).filter(|name| name.contains(needle)).count();
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

#[page("/admin/servers")]
pub(super) async fn servers(cx: &Cx) -> Result<impl View> {
    let guilds = list_bot_guilds(cx).await;

    Ok(view! { app_shell(servers_page(guilds: guilds)) })
}

#[component]
pub async fn servers_page(
    guilds: std::result::Result<Vec<GuildCard>, AdminError>,
) -> Result<impl View> {
    let guilds = guilds
        .map_err(|e| if e.is_denied() { None } else { Some(server_error_text(e)) });

    Ok(view! {
        <div class="page">
            <div class="page-header">
                <div>
                    <h1>"All Servers"</h1>
                    <p class="page-lead">
                        "Every server Zayden is in. Operator access ignores your own permissions in them."
                    </p>
                </div>
            </div>
            match guilds {
                Err(None) => <p class="error">
                    "Operator access is required to list every server."
                </p>,
                Err(Some(text)) => <p class="error">
                    "Couldn't load the server list: "
                    (text)
                </p>,
                Ok(guilds) => server_tools(guilds: &guilds),
            }
        </div>
    })
}

#[component]
async fn server_tools(cx: &Cx, guilds: &[GuildCard]) -> Result<impl View> {
    let total = guilds.len();
    let joined = guilds
        .iter()
        .map(|g| g.name.to_lowercase())
        .collect::<Vec<_>>()
        .join(SEPARATOR);

    let filter = signal(cx, String::new);
    let jump = signal(cx, String::new);

    let needle = expr!({
        let typed = filter.get();
        raw!(
            "cx.hydrate(${typed}.trim().toString().toLowerCase())",
            typed.trim().to_lowercase()
        )
    });
    let shown = expr!({
        let names = joined;
        let needle = needle;
        raw!(
            "cx.hydrate(${names}.toString() === '' ? 0 : ${names}.toString().split(String.fromCharCode(10)).filter((name) => name.includes(${needle}.toString())).length)",
            matching(&names, &needle)
        )
    });
    let target = expr!({
        let typed = jump.get();
        raw!(
            "cx.hydrate((() => { const t = ${typed}.trim().toString(); if (!/^[0-9]+$/.test(t)) return ''; const id = BigInt(t); return id > 0n && id <= 18446744073709551615n ? '/guild/' + id : ''; })())",
            parse_guild_id(&typed)
                .map_or_else(String::new, |id| format!("/guild/{id}"))
        )
    });

    Ok(view! {
        <div class="operator-tools">
            <input
                class="input"
                type="search"
                placeholder="Filter by name"
                aria-label="Filter servers by name"
                :value=$(filter.get())
                @input=$(|e: Event| filter.set(e.target.value))
            >
            <div class="operator-jump">
                <input
                    class="input"
                    type="text"
                    inputmode="numeric"
                    placeholder="Go to server ID"
                    aria-label="Go to server ID"
                    :value=$(jump.get())
                    @input=$(|e: Event| jump.set(e.target.value))
                >
                <button
                    type="button"
                    class="btn btn-secondary"
                    disabled=""
                    :hidden=$(!target.is_empty())
                >
                    "Go"
                </button>
                <a
                    class="btn btn-secondary"
                    :href=$(target)
                    :hidden=$(target.is_empty())
                >
                    "Go"
                </a>
            </div>
        </div>
        <p class="empty" :hidden=$(shown != 0.0)>"No server matches that name."</p>
        <p class="operator-count" :hidden=$(shown == 0.0)>
            $(shown)
            " of "
            (total)
            " servers"
        </p>
        <div class="guild-grid" :hidden=$(shown == 0.0)>
            #[key(guild.id.as_str())]
            for guild in guilds {
                let name = guild.name.to_lowercase();
                <a
                    href=(guild.href())
                    class="guild-card"
                    :hidden=$({
                        let name = name;
                        let needle = needle;
                        raw!(
                            "cx.hydrate(!${name}.toString().includes(${needle}.toString()))",
                            !name.contains(needle.as_str()),
                        )
                    })
                >
                    match guild.icon_url() {
                        Some(url) => <img src=(url) alt="" class="guild-icon">,
                        None => <span class="guild-icon placeholder">
                            (guild.initial())
                        </span>,
                    }
                    <div class="guild-card-body">
                        <div class="guild-name">(guild.name.as_str())</div>
                        <div class="guild-card-hint">"Manage \u{2192}"</div>
                    </div>
                </a>
            }
        </div>
    })
}
