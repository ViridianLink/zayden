#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
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
pub async fn guild_grid(guilds: &[GuildCard]) -> Result<impl View> {
    Ok(view! {
        <div class="guild-grid">
            #[key(guild.id.as_str())]
            for guild in guilds {
                <a href=(guild.href()) class="guild-card">
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
