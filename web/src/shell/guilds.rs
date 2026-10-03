use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::page;
use topcoat::view::{View, view};

use super::chrome::app_shell;
use crate::components::guild_grid::{GuildCard, guild_grid};
use crate::guild::list_manageable_guilds;
use crate::util::server_error_text;

/// The guilds the signed-in user manages. `list_manageable_guilds` keeps
/// only guilds where the user holds Administrator or Manage Server, before
/// they become cards.
#[page("/guilds")]
pub(super) async fn guilds(cx: &Cx) -> Result<impl View> {
    let guilds = match list_manageable_guilds(cx).await {
        Ok(guilds) => Ok(guilds.iter().map(GuildCard::from).collect::<Vec<_>>()),
        Err(e) => Err(server_error_text(e.redirect_unauthenticated()?)),
    };

    Ok(view! {
        app_shell(
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Your Servers"</h1>
                        <p class="page-lead">"Pick a server to configure Zayden."</p>
                    </div>
                    <a href="/invite" rel="external" class="btn btn-secondary">
                        "Add to a server"
                    </a>
                </div>
                match guilds {
                    Err(error) => <p class="error">
                        "Failed to load servers: "
                        (error)
                    </p>,
                    Ok(cards) => {
                        if cards.is_empty() {
                            <p class="empty">
                                "You manage no servers with this account."
                            </p>
                        } else {
                            guild_grid(guilds: &cards)
                        }
                    }
                }
            </div>
        )
    })
}
