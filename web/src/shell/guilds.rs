use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::page;
use topcoat::view::{View, view};

use super::chrome::app_shell;
use super::overview::plain;
use crate::components::empty_state::empty_state;
use crate::components::error_panel::{ErrorAction, error_panel};
use crate::components::guild_grid::{GuildCard, guild_grid};
use crate::guild::list_manageable_guilds;

#[page("/guilds")]
pub(super) async fn guilds(cx: &Cx) -> Result<impl View> {
    let guilds = match list_manageable_guilds(cx).await {
        Ok(guilds) => Ok(guilds.iter().map(GuildCard::from).collect::<Vec<_>>()),
        Err(e) => Err(format!(
            "Your server list couldn't be loaded: {}",
            plain(&e.redirect_unauthenticated()?.to_string())
        )),
    };
    let retry = [
        ErrorAction::new("Try again", "/guilds"),
        ErrorAction::new("Add Zayden to a server", "/invite").external(),
    ];

    Ok(view! {
        app_shell(
            <div class="page">
                match guilds {
                    Err(error) => error_panel(
                        title: "Couldn't load your servers",
                        message: &error,
                        actions: &retry
                    ),
                    Ok(cards) => {
                        <div class="page-header">
                            <div>
                                <h1>"Servers"</h1>
                                <p class="page-lead">"Pick a server to configure Zayden."</p>
                            </div>
                            if !cards.is_empty() {
                                <a href="/invite" rel="external" class="btn btn-secondary">
                                    "Add Zayden to a server"
                                </a>
                            }
                        </div>
                        if cards.is_empty() {
                            empty_state(
                                title: "Add Zayden to a server you manage",
                                text: "Servers appear here when you have Manage Server in them.",
                                action: Some("Add Zayden to a server"),
                                href: Some("/invite"),
                                external: true
                            )
                        } else {
                            guild_grid(guilds: &cards)
                        }
                    }
                }
            </div>
        )
    })
}
