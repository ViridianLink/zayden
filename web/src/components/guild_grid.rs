#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{PrefetchMode, link_attrs};
use topcoat::view::{View, component, view};
use twilight_model::user::CurrentUserGuild;

use crate::guild::dto::GuildInfo;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildCard {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

impl From<&CurrentUserGuild> for GuildCard {
    fn from(guild: &CurrentUserGuild) -> Self {
        Self {
            id: guild.id.to_string(),
            name: guild.name.clone(),
            icon: guild.icon.map(|hash| hash.to_string()),
        }
    }
}

impl From<&GuildInfo> for GuildCard {
    fn from(guild: &GuildInfo) -> Self {
        Self {
            id: guild.id.clone(),
            name: guild.name.clone(),
            icon: guild.icon.clone(),
        }
    }
}

impl GuildCard {
    #[must_use]
    pub fn href(&self) -> String {
        format!("/guild/{}", self.id)
    }

    #[must_use]
    pub fn icon_url(&self) -> Option<String> {
        self.icon.as_ref().map(|hash| {
            format!(
                "https://cdn.discordapp.com/icons/{}/{hash}.png?size=64",
                self.id
            )
        })
    }

    #[must_use]
    pub fn initial(&self) -> String {
        self.name.chars().next().unwrap_or('#').to_string()
    }
}

#[component]
pub async fn guild_grid(cx: &Cx, guilds: &[GuildCard]) -> Result<impl View> {
    Ok(view! {
        <div class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4">
            #[key(guild.id.as_str())]
            for guild in guilds {
                <a
                    class="guild-card"
                    (link_attrs(cx, guild.href(), PrefetchMode::Intent))
                >
                    match guild.icon_url() {
                        Some(url) => <img
                            src=(url)
                            alt=""
                            class="h-[48px] w-[48px] shrink-0 rounded-control object-cover"
                        >,
                        None => <span
                            class="grid h-[48px] w-[48px] shrink-0 place-items-center rounded-control border border-solid border-line bg-surface-2 object-cover text-[18px] font-semibold text-text"
                        >
                            (guild.initial())
                        </span>,
                    }
                    <div class="min-w-0">
                        <div class="truncate font-semibold">(guild.name.as_str())</div>
                        <div class="text-[12.5px] text-text-3">"Manage \u{2192}"</div>
                    </div>
                </a>
            }
        </div>
    })
}
